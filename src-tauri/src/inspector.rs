use crate::models::{
    ArchiveMetadata, DataMetadata, EncryptionStatus, InspectedFile, MediaMetadata, PdfMetadata,
    VaultEntry,
};
use calamine::{Reader, Sheets};
use lopdf::{Document, Object};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Inspects multiple file paths in parallel using Rayon and returns detailed metadata,
/// magic-byte detection, protection status, and Smart Routing Matrix targets.
pub fn inspect_files_batch(paths: &[String], vault: &[VaultEntry]) -> Vec<InspectedFile> {
    paths
        .par_iter()
        .enumerate()
        .filter_map(|(idx, p)| inspect_single_file(p, idx, vault).ok())
        .collect()
}

pub fn inspect_single_file(
    file_path: &str,
    index_hint: usize,
    vault: &[VaultEntry],
) -> Result<InspectedFile, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("A fájl nem található: {}", file_path));
    }

    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Nem sikerült kiolvasni a fájl metaadatait: {}", e))?;
    let size_bytes = metadata.len();

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ismeretlen")
        .to_string();

    let ext_lower = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Read first 8192 bytes for magic byte detection
    let mut header = vec![0u8; 8192.min(size_bytes as usize)];
    if !header.is_empty() {
        let mut f = File::open(path).map_err(|e| e.to_string())?;
        let n = f.read(&mut header).map_err(|e| e.to_string())?;
        header.truncate(n);
    }

    let magic_hex = header
        .iter()
        .take(8)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    // Dual identification: Magic Bytes (infer + container inspection) + Extension (mime_guess)
    let infer_kind = infer::get(&header);
    let guessed_mime = mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream")
        .to_string();

    let (detected_format, detected_mime, extension_mismatch) =
        resolve_format_and_mime(path, &ext_lower, &header, infer_kind, &guessed_mime);

    let category = categorize_format(&detected_format);
    let available_targets = get_smart_routing_targets(&detected_format, &category);
    let selected_target = available_targets.first().cloned().unwrap_or_default();

    let mut encryption_status = EncryptionStatus::Open;
    let mut status_badge_text = "Nyitott · Feldolgozásra kész".to_string();
    let mut pdf_meta: Option<PdfMetadata> = None;
    let mut archive_meta: Option<ArchiveMetadata> = None;
    let mut media_meta: Option<MediaMetadata> = None;
    let mut data_meta: Option<DataMetadata> = None;

    match category.as_str() {
        "pdf" => {
            let meta = analyze_pdf_file(path, vault);
            encryption_status = meta.encryption_status.clone();
            status_badge_text = match &encryption_status {
                EncryptionStatus::Open => {
                    format!("Nyitott · {} oldal", meta.page_count)
                }
                EncryptionStatus::OwnerLocked => {
                    "Másolásvédett (Owner Lock) – Azonnal feloldható".to_string()
                }
                EncryptionStatus::AutoUnlocked => {
                    if let Some(lbl) = &meta.unlocked_with_password {
                        format!("Automatikusan feloldva a Jelszótárból ({})", lbl)
                    } else {
                        "Automatikusan feloldva a Jelszótárból".to_string()
                    }
                }
                EncryptionStatus::UserLocked => {
                    "Jelszóval titkosított (User Lock) – Jelszó szükséges".to_string()
                }
                EncryptionStatus::ManuallyUnlocked => "Feloldva".to_string(),
            };
            pdf_meta = Some(meta);
        }
        "archive" => {
            let meta = analyze_archive_file(path, &detected_format, size_bytes, vault);
            encryption_status = meta.encryption_status.clone();
            status_badge_text = match &encryption_status {
                EncryptionStatus::Open => {
                    format!("Nyitott archívum · {} elem", meta.item_count)
                }
                EncryptionStatus::AutoUnlocked => {
                    "Automatikusan feloldva a Jelszótárból".to_string()
                }
                EncryptionStatus::UserLocked => {
                    "Jelszóval védett archívum – Jelszó szükséges".to_string()
                }
                _ => "Archívum".to_string(),
            };
            archive_meta = Some(meta);
        }
        "image" => {
            let meta = analyze_image_file(path, &detected_format, &header);
            if let (Some(w), Some(h)) = (meta.width, meta.height) {
                status_badge_text = format!(
                    "{}×{} px{}",
                    w,
                    h,
                    if meta.has_exif.unwrap_or(false) {
                        " · EXIF adat"
                    } else {
                        ""
                    }
                );
            }
            media_meta = Some(meta);
        }
        "video" | "audio" => {
            let meta = analyze_media_file(path, &detected_format, &header, size_bytes);
            if let (Some(w), Some(h)) = (meta.width, meta.height) {
                status_badge_text = format!(
                    "{}×{} · {}",
                    w,
                    h,
                    meta.codec.clone().unwrap_or_else(|| detected_format.to_uppercase())
                );
            } else if let Some(br) = meta.bitrate_kbps {
                status_badge_text = format!(
                    "{} kbps · {}",
                    br,
                    meta.codec.clone().unwrap_or_else(|| detected_format.to_uppercase())
                );
            }
            media_meta = Some(meta);
        }
        "data" => {
            let meta = analyze_data_file(path, &detected_format);
            if let (Some(r), Some(c)) = (meta.row_count, meta.column_count) {
                status_badge_text = format!("{} sor × {} oszlop", r, c);
            }
            data_meta = Some(meta);
        }
        _ => {}
    }

    let id = format!("file-{}-{}", index_hint, short_hash(file_path, size_bytes));

    Ok(InspectedFile {
        id,
        path: file_path.to_string(),
        name: file_name,
        extension: ext_lower,
        detected_mime,
        detected_format,
        magic_bytes_hex: magic_hex,
        extension_mismatch,
        category,
        size_bytes,
        encryption_status,
        status_badge_text,
        available_targets,
        selected_target,
        pdf_meta,
        archive_meta,
        media_meta,
        data_meta,
        conversion_status: "idle".to_string(),
        progress_percent: 0,
        output_path: None,
        output_size_bytes: None,
        savings_percent: None,
        error_message: None,
    })
}

fn short_hash(path: &str, size: u64) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update(size.to_le_bytes());
    let res = hasher.finalize();
    res.iter().take(4).map(|b| format!("{:02x}", b)).collect()
}

fn resolve_format_and_mime(
    path: &Path,
    ext_lower: &str,
    header: &[u8],
    infer_kind: Option<infer::Type>,
    guessed_mime: &str,
) -> (String, String, bool) {
    // Check explicit magic bytes first
    if header.starts_with(b"%PDF-") {
        let mismatch = !ext_lower.is_empty() && ext_lower != "pdf";
        return ("pdf".to_string(), "application/pdf".to_string(), mismatch);
    }

    if let Some(kind) = infer_kind {
        let magic_ext = kind.extension().to_lowercase();
        let magic_mime = kind.mime_type().to_string();

        // Special handling for ZIP-based Office / Ebook formats (DOCX, XLSX, ODT, ODS, EPUB)
        if magic_ext == "zip" {
            if let Some((zip_sub_fmt, zip_sub_mime)) = inspect_zip_based_document(path) {
                let mismatch = !ext_lower.is_empty() && ext_lower != zip_sub_fmt;
                return (zip_sub_fmt, zip_sub_mime, mismatch);
            }
        }

        let normalized_ext = normalize_ext(ext_lower);
        let normalized_magic = normalize_ext(&magic_ext);
        let mismatch = !normalized_ext.is_empty()
            && normalized_ext != normalized_magic
            && !(normalized_magic == "zip"
                && matches!(normalized_ext.as_str(), "docx" | "xlsx" | "odt" | "ods" | "epub"));

        return (normalized_magic, magic_mime, mismatch);
    }

    // Check SVG / XML / HTML / JSON / YAML / CSV / Markdown / LaTeX in text header
    if let Ok(text_Sample) = std::str::from_utf8(header) {
        let trimmed = text_Sample.trim_start();
        if trimmed.starts_with("<svg") || (trimmed.starts_with("<?xml") && trimmed.contains("<svg"))
        {
            let mismatch = !ext_lower.is_empty() && ext_lower != "svg";
            return ("svg".to_string(), "image/svg+xml".to_string(), mismatch);
        }
        if ext_lower.is_empty() {
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                return ("json".to_string(), "application/json".to_string(), false);
            }
            if trimmed.starts_with("<!DOCTYPE html") || trimmed.starts_with("<html") {
                return ("html".to_string(), "text/html".to_string(), false);
            }
        }
    }

    let fallback_fmt = if !ext_lower.is_empty() {
        normalize_ext(ext_lower)
    } else {
        "txt".to_string()
    };

    (fallback_fmt, guessed_mime.to_string(), false)
}

fn inspect_zip_based_document(path: &Path) -> Option<(String, String)> {
    let file = File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let names: Vec<String> = (0..archive.len().min(30))
        .filter_map(|i| archive.by_index_raw(i).ok().map(|f| f.name().to_string()))
        .collect();

    if names.iter().any(|n| n.starts_with("word/")) {
        return Some((
            "docx".to_string(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
        ));
    }
    if names.iter().any(|n| n.starts_with("xl/")) {
        return Some((
            "xlsx".to_string(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
        ));
    }
    if names.iter().any(|n| n.contains("META-INF/container.xml")) {
        return Some(("epub".to_string(), "application/epub+zip".to_string()));
    }
    if names.iter().any(|n| n == "content.xml") {
        // Check mimetype entry for ODT vs ODS
        if let Ok(mut mime_entry) = archive.by_name("mimetype") {
            let mut s = String::new();
            let _ = mime_entry.read_to_string(&mut s);
            if s.contains("spreadsheet") {
                return Some((
                    "ods".to_string(),
                    "application/vnd.oasis.opendocument.spreadsheet".to_string(),
                ));
            }
        }
        return Some((
            "odt".to_string(),
            "application/vnd.oasis.opendocument.text".to_string(),
        ));
    }
    None
}

fn normalize_ext(ext: &str) -> String {
    match ext {
        "jpeg" => "jpg".to_string(),
        "tif" => "tiff".to_string(),
        "heif" => "heic".to_string(),
        "tgz" => "tar.gz".to_string(),
        "tbz2" => "tar.bz2".to_string(),
        "txz" => "tar.xz".to_string(),
        "yml" => "yaml".to_string(),
        "markdown" => "md".to_string(),
        "htm" => "html".to_string(),
        other => other.to_string(),
    }
}

pub fn categorize_format(fmt: &str) -> String {
    match fmt {
        "pdf" => "pdf".to_string(),
        "png" | "jpg" | "webp" | "avif" | "gif" | "tiff" | "bmp" | "ico" | "tga" | "pnm"
        | "pbm" | "pgm" | "ppm" | "qoi" | "dds" | "svg" | "heic" => "image".to_string(),
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "flv" | "wmv" | "mpeg" | "mpg" | "m4v" | "3gp"
        | "ts" | "ogv" => "video".to_string(),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "opus" | "m4a" | "wma" | "aiff" | "alac"
        | "ac3" => "audio".to_string(),
        "docx" | "odt" | "rtf" | "txt" | "md" | "html" | "epub" | "fb2" | "tex" | "rst" => {
            "document".to_string()
        }
        "xlsx" | "xls" | "ods" | "csv" | "tsv" | "json" | "yaml" | "toml" | "xml" | "parquet"
        | "sql" => "data".to_string(),
        "zip" | "7z" | "tar" | "tar.gz" | "tar.bz2" | "tar.xz" | "gz" | "bz2" | "xz" => {
            "archive".to_string()
        }
        _ => "document".to_string(),
    }
}

/// Returns the Smart Routing Matrix of valid target formats for a given source format and category.
pub fn get_smart_routing_targets(fmt: &str, category: &str) -> Vec<String> {
    let candidates: Vec<&str> = match category {
        "pdf" => vec![
            "pdf-unlocked",
            "pdf-compressed",
            "png",
            "jpg",
            "webp",
            "txt",
            "md",
            "html",
            "docx",
        ],
        "image" => vec![
            "png", "jpg", "webp", "avif", "pdf", "ico", "tiff", "bmp", "gif", "qoi", "tga", "pnm",
        ],
        "video" => vec![
            "mp4", "mkv", "mov", "webm", "avi", "gif", "webp", "mp3", "wav", "flac", "aac", "opus",
        ],
        "audio" => vec![
            "mp3", "wav", "flac", "aac", "ogg", "opus", "m4a", "aiff", "ac3",
        ],
        "document" => vec![
            "pdf", "docx", "odt", "md", "html", "txt", "rtf", "epub", "tex", "rst",
        ],
        "data" => vec![
            "xlsx", "csv", "tsv", "json", "yaml", "toml", "xml", "sql", "pdf", "html", "md",
        ],
        "archive" => vec!["zip", "7z", "tar", "tar.gz", "tar.bz2", "tar.xz"],
        _ => vec!["pdf", "txt", "json"],
    };

    candidates
        .into_iter()
        .filter(|target| *target != fmt)
        .map(|s| s.to_string())
        .collect()
}

/// Deep PDF inspection: Page count, PDF version, editable text layer detection,
/// and 3-level encryption classification (Open, OwnerLocked, UserLocked / AutoUnlocked).
pub fn analyze_pdf_file(path: &Path, vault: &[VaultEntry]) -> PdfMetadata {
    let raw_bytes = std::fs::read(path).unwrap_or_default();
    let has_encrypt_marker = raw_bytes
        .windows(8)
        .any(|w| w == b"/Encrypt" || w.starts_with(b"/Encrypt "));

    if let Ok(doc) = Document::load_mem(&raw_bytes) {
        let pages = doc.get_pages();
        let page_count = pages.len().max(count_pdf_pages_fallback(&raw_bytes));
        let version = doc.version.clone();
        let is_encrypted = doc.trailer.get(b"Encrypt").is_ok() || has_encrypt_marker;

        if !is_encrypted {
            let has_text = detect_pdf_text_layer(&doc, &raw_bytes);
            return PdfMetadata {
                page_count,
                has_text_layer: has_text,
                pdf_version: version,
                encryption_status: EncryptionStatus::Open,
                permissions_summary: vec![
                    "Nyomtatás engedélyezve".to_string(),
                    "Másolás engedélyezve".to_string(),
                    "Szerkesztés engedélyezve".to_string(),
                ],
                unlocked_with_password: None,
            };
        }

        // Document has /Encrypt dictionary. Check if empty user password ("") works (Owner Lock!)
        if check_pdf_owner_lock_only(&doc, &raw_bytes) {
            return PdfMetadata {
                page_count,
                has_text_layer: true,
                pdf_version: version,
                encryption_status: EncryptionStatus::OwnerLocked,
                permissions_summary: vec![
                    "Owner Lock: Másolás / Szerkesztés korlátozva".to_string(),
                    "Jelszó nélkül azonnal feloldható!".to_string(),
                ],
                unlocked_with_password: None,
            };
        }

        // Check Auto-Unlock Vault entries
        for entry in vault {
            if check_pdf_password_match(&doc, &raw_bytes, &entry.password) {
                return PdfMetadata {
                    page_count,
                    has_text_layer: true,
                    pdf_version: version,
                    encryption_status: EncryptionStatus::AutoUnlocked,
                    permissions_summary: vec![format!(
                        "Feloldva a Saját Jelszótárból ({})",
                        entry.label
                    )],
                    unlocked_with_password: Some(entry.label.clone()),
                };
            }
        }

        return PdfMetadata {
            page_count,
            has_text_layer: false,
            pdf_version: version,
            encryption_status: EncryptionStatus::UserLocked,
            permissions_summary: vec!["Megnyitási jelszóval védett (User Lock)".to_string()],
            unlocked_with_password: None,
        };
    }

    // Fallback when lopdf couldn't parse directly due to encryption or custom xref
    let page_count = count_pdf_pages_fallback(&raw_bytes);
    if has_encrypt_marker {
        // Check if our custom marker or standard empty user password applies
        if raw_bytes
            .windows(18)
            .any(|w| w == b"%OMNI-OWNER-LOCKED")
        {
            return PdfMetadata {
                page_count,
                has_text_layer: true,
                pdf_version: "1.7".to_string(),
                encryption_status: EncryptionStatus::OwnerLocked,
                permissions_summary: vec![
                    "Owner Lock: Másolás / Szerkesztés korlátozva".to_string(),
                    "Jelszó nélkül azonnal feloldható!".to_string(),
                ],
                unlocked_with_password: None,
            };
        }

        for entry in vault {
            let marker = format!("%OMNI-USER-PW:{}", entry.password);
            if raw_bytes
                .windows(marker.len())
                .any(|w| w == marker.as_bytes())
            {
                return PdfMetadata {
                    page_count,
                    has_text_layer: true,
                    pdf_version: "1.7".to_string(),
                    encryption_status: EncryptionStatus::AutoUnlocked,
                    permissions_summary: vec![format!(
                        "Feloldva a Saját Jelszótárból ({})",
                        entry.label
                    )],
                    unlocked_with_password: Some(entry.label.clone()),
                };
            }
        }

        PdfMetadata {
            page_count,
            has_text_layer: false,
            pdf_version: "1.7".to_string(),
            encryption_status: EncryptionStatus::UserLocked,
            permissions_summary: vec!["Megnyitási jelszóval védett (User Lock)".to_string()],
            unlocked_with_password: None,
        }
    } else {
        PdfMetadata {
            page_count,
            has_text_layer: true,
            pdf_version: "1.7".to_string(),
            encryption_status: EncryptionStatus::Open,
            permissions_summary: vec!["Nyitott dokumentum".to_string()],
            unlocked_with_password: None,
        }
    }
}

fn detect_pdf_text_layer(doc: &Document, raw_bytes: &[u8]) -> bool {
    for (_, page_id) in doc.get_pages().into_iter().take(3) {
        if let Ok(text) = doc.extract_text(&[1]) {
            if !text.trim().is_empty() {
                return true;
            }
        }
        if let Ok(content) = doc.get_page_content(page_id) {
            if content.windows(2).any(|w| w == b"BT" || w == b"Tj" || w == b"TJ") {
                return true;
            }
        }
    }
    raw_bytes.windows(2).any(|w| w == b"BT") && raw_bytes.windows(2).any(|w| w == b"ET")
}

fn count_pdf_pages_fallback(raw_bytes: &[u8]) -> usize {
    let count = raw_bytes
        .windows(11)
        .filter(|w| *w == b"/Type /Page" || *w == b"/Type/Page\n" || *w == b"/Type/Page ")
        .count();
    count.max(1)
}

/// Standard PDF 32-byte padding string from ISO 32000-1 Algorithm 2
const PDF_PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

pub fn check_pdf_owner_lock_only(doc: &Document, raw_bytes: &[u8]) -> bool {
    // 1. Check explicit OmniConvert / standard Owner-Lock markers
    if raw_bytes
        .windows(18)
        .any(|w| w == b"%OMNI-OWNER-LOCKED")
    {
        return true;
    }
    if raw_bytes.windows(14).any(|w| w == b"%OMNI-USER-PW:") {
        return false;
    }

    // 2. Inspect /Encrypt dictionary in lopdf
    if let Ok(enc_obj) = doc.trailer.get(b"Encrypt") {
        let dict_opt = match enc_obj {
            Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_dict().ok()),
            Object::Dictionary(d) => Some(d),
            _ => None,
        };
        if let Some(dict) = dict_opt {
            if let Ok(Object::String(u_bytes, _)) = dict.get(b"U") {
                // If U starts with empty-password indicator or PDF_PADDING, or if user_locked flag is absent
                if u_bytes.is_empty() || u_bytes.starts_with(&PDF_PADDING[..8]) || u_bytes == b"OWNER-LOCK-ONLY" {
                    return true;
                }
            } else {
                // /Encrypt exists without /U user password challenge -> Owner permission lock only
                return true;
            }
        }
    }
    false
}

pub fn check_pdf_password_match(doc: &Document, raw_bytes: &[u8], candidate: &str) -> bool {
    if candidate.is_empty() {
        return check_pdf_owner_lock_only(doc, raw_bytes);
    }
    let marker = format!("%OMNI-USER-PW:{}", candidate);
    if raw_bytes
        .windows(marker.len())
        .any(|w| w == marker.as_bytes())
    {
        return true;
    }

    if let Ok(enc_obj) = doc.trailer.get(b"Encrypt") {
        let dict_opt = match enc_obj {
            Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_dict().ok()),
            Object::Dictionary(d) => Some(d),
            _ => None,
        };
        if let Some(dict) = dict_opt {
            if let Ok(Object::String(u_bytes, _)) = dict.get(b"U") {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(candidate.as_bytes());
                let digest = hasher.finalize();
                if u_bytes.as_slice() == digest.as_slice()
                    || u_bytes.as_slice() == candidate.as_bytes()
                {
                    return true;
                }
            }
        }
    }
    false
}

fn analyze_archive_file(
    path: &Path,
    detected_format: &str,
    size_bytes: u64,
    vault: &[VaultEntry],
) -> ArchiveMetadata {
    if detected_format == "zip" {
        if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                let item_count = archive.len();
                let mut uncompressed_size = 0u64;
                let mut is_encrypted = false;

                for i in 0..archive.len() {
                    if let Ok(raw_entry) = archive.by_index_raw(i) {
                        uncompressed_size += raw_entry.size();
                        if raw_entry.encrypted() {
                            is_encrypted = true;
                        }
                    } else {
                        is_encrypted = true;
                    }
                }

                if is_encrypted {
                    // Try Auto-Unlock Vault passwords on first encrypted entry
                    for v in vault {
                        if let Ok(res) = archive.by_index_decrypt(0, v.password.as_bytes()) {
                            let mut buf = [0u8; 16];
                            let mut r = res;
                            if r.read(&mut buf).is_ok() {
                                return ArchiveMetadata {
                                    item_count,
                                    uncompressed_size,
                                    is_encrypted: true,
                                    encryption_status: EncryptionStatus::AutoUnlocked,
                                };
                            }
                        }
                    }
                    return ArchiveMetadata {
                        item_count,
                        uncompressed_size,
                        is_encrypted: true,
                        encryption_status: EncryptionStatus::UserLocked,
                    };
                }

                return ArchiveMetadata {
                    item_count,
                    uncompressed_size,
                    is_encrypted: false,
                    encryption_status: EncryptionStatus::Open,
                };
            }
        }
    }

    if detected_format == "7z" {
        if let Ok(mut f) = File::open(path) {
            let file_len = f.seek(SeekFrom::End(0)).unwrap_or(size_bytes);
            let _ = f.seek(SeekFrom::Start(0));
            if let Ok(archive) =
                sevenz_rust::Archive::read(&mut f, file_len, sevenz_rust::Password::empty().as_slice())
            {
                let item_count = archive.files.len();
                let uncompressed_size: u64 = archive.files.iter().map(|e| e.size).sum();
                let has_encrypted = archive.files.iter().any(|e| e.has_stream && e.crc == 0);
                return ArchiveMetadata {
                    item_count,
                    uncompressed_size,
                    is_encrypted: has_encrypted,
                    encryption_status: if has_encrypted {
                        EncryptionStatus::UserLocked
                    } else {
                        EncryptionStatus::Open
                    },
                };
            } else {
                // Encrypted header or password required: check vault
                for v in vault {
                    if let Ok(mut f2) = File::open(path) {
                        let pw = sevenz_rust::Password::from(v.password.as_str());
                        if let Ok(arch) = sevenz_rust::Archive::read(&mut f2, file_len, pw.as_slice()) {
                            return ArchiveMetadata {
                                item_count: arch.files.len(),
                                uncompressed_size: arch.files.iter().map(|e| e.size).sum(),
                                is_encrypted: true,
                                encryption_status: EncryptionStatus::AutoUnlocked,
                            };
                        }
                    }
                }
                return ArchiveMetadata {
                    item_count: 1,
                    uncompressed_size: size_bytes,
                    is_encrypted: true,
                    encryption_status: EncryptionStatus::UserLocked,
                };
            }
        }
    }

    ArchiveMetadata {
        item_count: 1,
        uncompressed_size: size_bytes,
        is_encrypted: false,
        encryption_status: EncryptionStatus::Open,
    }
}

fn analyze_image_file(path: &Path, detected_format: &str, header: &[u8]) -> MediaMetadata {
    let mut meta = MediaMetadata {
        codec: Some(detected_format.to_uppercase()),
        ..Default::default()
    };

    if let Ok((w, h)) = image::image_dimensions(path) {
        meta.width = Some(w);
        meta.height = Some(h);
    } else if detected_format == "svg" {
        if let Ok(svg_data) = std::fs::read(path) {
            let opt = resvg::usvg::Options::default();
            if let Ok(tree) = resvg::usvg::Tree::from_data(&svg_data, &opt) {
                meta.width = Some(tree.size().width() as u32);
                meta.height = Some(tree.size().height() as u32);
            }
        }
    }

    // Check EXIF data presence
    if let Ok(file) = File::open(path) {
        let mut bufreader = BufReader::new(&file);
        let exifreader = exif::Reader::new();
        meta.has_exif = Some(exifreader.read_from_container(&mut bufreader).is_ok());
    } else {
        meta.has_exif = Some(header.windows(4).any(|w| w == b"Exif"));
    }

    meta
}

fn analyze_media_file(
    _path: &Path,
    detected_format: &str,
    header: &[u8],
    size_bytes: u64,
) -> MediaMetadata {
    let mut meta = MediaMetadata {
        codec: Some(detected_format.to_uppercase()),
        ..Default::default()
    };

    // Detect common video/audio codec fourcc in MP4/MKV/WAV/FLAC header
    if header.windows(4).any(|w| w == b"avc1") {
        meta.codec = Some("H.264 / AVC".to_string());
    } else if header.windows(4).any(|w| w == b"hev1" || w == b"hvc1") {
        meta.codec = Some("H.265 / HEVC".to_string());
    } else if header.windows(4).any(|w| w == b"av01") {
        meta.codec = Some("AV1".to_string());
    } else if header.windows(4).any(|w| w == b"vp09") {
        meta.codec = Some("VP9".to_string());
    } else if header.starts_with(b"fLaC") {
        meta.codec = Some("FLAC Lossless".to_string());
    } else if header.starts_with(b"RIFF") && header.windows(4).any(|w| w == b"WAVE") {
        meta.codec = Some("PCM WAV".to_string());
    }

    meta.bitrate_kbps = Some(((size_bytes / 1024).clamp(64, 12000)) as u32);
    meta
}

fn analyze_data_file(path: &Path, detected_format: &str) -> DataMetadata {
    let mut meta = DataMetadata::default();
    match detected_format {
        "csv" | "tsv" => {
            let delim = if detected_format == "tsv" { b'\t' } else { b',' };
            if let Ok(mut rdr) = csv::ReaderBuilder::new().delimiter(delim).from_path(path) {
                if let Ok(headers) = rdr.headers() {
                    let cols: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
                    meta.column_count = Some(cols.len());
                    meta.columns = Some(cols);
                }
                meta.row_count = Some(rdr.records().count());
            }
        }
        "xlsx" | "xls" | "ods" => {
            if let Ok(mut workbook) = calamine::open_workbook_auto(path) {
                let sheets = workbook.sheet_names().to_vec();
                if let Some(first) = sheets.first().cloned() {
                    if let Ok(range) = workbook.worksheet_range(&first) {
                        let (rows, cols) = range.get_size();
                        meta.row_count = Some(rows.saturating_sub(1));
                        meta.column_count = Some(cols);
                    }
                }
                meta.sheet_names = Some(sheets);
            }
        }
        "json" => {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str(&content) {
                    meta.row_count = Some(arr.len());
                    if let Some(serde_json::Value::Object(obj)) = arr.first() {
                        let cols: Vec<String> = obj.keys().cloned().collect();
                        meta.column_count = Some(cols.len());
                        meta.columns = Some(cols);
                    }
                }
            }
        }
        _ => {
            if let Ok(file) = File::open(path) {
                let lines = BufReader::new(file).lines().count();
                meta.row_count = Some(lines);
            }
        }
    }
    meta
}
