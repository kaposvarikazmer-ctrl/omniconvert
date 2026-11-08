use crate::converters::archive_conv::{extract_archive_to_dir, pack_dir_to_archive};
use crate::converters::image_conv::ensure_output_dir;
use crate::inspector::{check_pdf_owner_lock_only, check_pdf_password_match};
use crate::models::{EncryptionStatus, UnlockResult, VaultEntry};
use lopdf::{Document, Object};
use std::path::{Path, PathBuf};

/// 3-Tier FileUnlocker:
/// 1. Instant Owner/Permission Lock Stripping without password (PDF)
/// 2. Auto-Unlock Keyring (Vault) password matching (PDF, ZIP, 7Z)
/// 3. Permanent Decrypt & Save with manual password (PDF, ZIP, 7Z)
pub fn unlock_file_permanent(
    file_id: &str,
    input_path_str: &str,
    output_dir: &str,
    manual_password: Option<&str>,
    vault_entries: &[VaultEntry],
) -> Result<UnlockResult, String> {
    let input_path = Path::new(input_path_str);
    if !input_path.exists() {
        return Err(format!("A védett fájl nem található: {}", input_path_str));
    }

    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" => unlock_pdf_file(
            file_id,
            input_path,
            output_dir,
            manual_password,
            vault_entries,
        ),
        "zip" | "7z" => unlock_archive_file(
            file_id,
            input_path,
            &ext,
            output_dir,
            manual_password,
            vault_entries,
        ),
        _ => {
            // Check magic bytes for PDF even if extension differs
            let raw = std::fs::read(input_path).unwrap_or_default();
            if raw.starts_with(b"%PDF-") {
                unlock_pdf_file(
                    file_id,
                    input_path,
                    output_dir,
                    manual_password,
                    vault_entries,
                )
            } else {
                Err(format!(
                    "A fájltípus ({}) nem támogatja a jelszóeltávolítást.",
                    ext
                ))
            }
        }
    }
}

fn unlock_pdf_file(
    file_id: &str,
    input_path: &Path,
    output_dir: &str,
    manual_password: Option<&str>,
    vault_entries: &[VaultEntry],
) -> Result<UnlockResult, String> {
    let raw_bytes = std::fs::read(input_path).map_err(|e| e.to_string())?;
    let out_dir = ensure_output_dir(output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let out_path = out_dir.join(format!("{}_unlocked.pdf", stem));

    let mut doc = Document::load_mem(&raw_bytes)
        .map_err(|e| format!("PDF struktúra olvasási hiba: {}", e))?;

    // Tier 1: Owner Lock / Permission Lock only -> Instant passwordless stripping!
    if check_pdf_owner_lock_only(&doc, &raw_bytes) {
        strip_pdf_encryption_structures(&mut doc);
        save_clean_pdf(&mut doc, &raw_bytes, &out_path)?;

        return Ok(UnlockResult {
            file_id: file_id.to_string(),
            input_path: input_path.to_string_lossy().to_string(),
            success: true,
            output_path: Some(out_path.to_string_lossy().to_string()),
            new_status: EncryptionStatus::Open,
            method_used: "owner_lock_strip".to_string(),
            matched_label: None,
            message: "Owner Lock (másolási/szerkesztési korlátozás) jelszó nélkül azonnal eltávolítva!"
                .to_string(),
        });
    }

    // Tier 2: Manual password if provided by user
    if let Some(pw) = manual_password.filter(|s| !s.is_empty()) {
        if check_pdf_password_match(&doc, &raw_bytes, pw) {
            strip_pdf_encryption_structures(&mut doc);
            save_clean_pdf(&mut doc, &raw_bytes, &out_path)?;

            return Ok(UnlockResult {
                file_id: file_id.to_string(),
                input_path: input_path.to_string_lossy().to_string(),
                success: true,
                output_path: Some(out_path.to_string_lossy().to_string()),
                new_status: EncryptionStatus::ManuallyUnlocked,
                method_used: "manual_password".to_string(),
                matched_label: None,
                message: "PDF sikeresen feloldva és véglegesen jelszómentesítve!".to_string(),
            });
        }
    }

    // Tier 3: Auto-Unlock Vault (Saját Jelszótár)
    for entry in vault_entries {
        if check_pdf_password_match(&doc, &raw_bytes, &entry.password) {
            strip_pdf_encryption_structures(&mut doc);
            save_clean_pdf(&mut doc, &raw_bytes, &out_path)?;

            return Ok(UnlockResult {
                file_id: file_id.to_string(),
                input_path: input_path.to_string_lossy().to_string(),
                success: true,
                output_path: Some(out_path.to_string_lossy().to_string()),
                new_status: EncryptionStatus::AutoUnlocked,
                method_used: "auto_vault".to_string(),
                matched_label: Some(entry.label.clone()),
                message: format!(
                    "Automatikusan feloldva a Saját Jelszótárból ({}) és jelszómentesítve!",
                    entry.label
                ),
            });
        }
    }

    // Check if document wasn't encrypted at all
    if doc.trailer.get(b"Encrypt").is_err()
        && !raw_bytes.windows(8).any(|w| w == b"/Encrypt")
    {
        std::fs::copy(input_path, &out_path).map_err(|e| e.to_string())?;
        return Ok(UnlockResult {
            file_id: file_id.to_string(),
            input_path: input_path.to_string_lossy().to_string(),
            success: true,
            output_path: Some(out_path.to_string_lossy().to_string()),
            new_status: EncryptionStatus::Open,
            method_used: "already_open".to_string(),
            matched_label: None,
            message: "A PDF már nyitott állapotban van.".to_string(),
        });
    }

    Ok(UnlockResult {
        file_id: file_id.to_string(),
        input_path: input_path.to_string_lossy().to_string(),
        success: false,
        output_path: None,
        new_status: EncryptionStatus::UserLocked,
        method_used: "manual_password".to_string(),
        matched_label: None,
        message: "A megadott jelszó nem nyitja ezt a PDF-et, és a Saját Jelszótárban sem található egyezés."
            .to_string(),
    })
}

fn strip_pdf_encryption_structures(doc: &mut Document) {
    if let Ok(Object::Reference(enc_id)) = doc.trailer.get(b"Encrypt").cloned() {
        doc.objects.remove(&enc_id);
    }
    doc.trailer.remove(b"Encrypt");
}

fn save_clean_pdf(doc: &mut Document, raw_bytes: &[u8], out_path: &Path) -> Result<(), String> {
    doc.prune_objects();
    if doc.save(out_path).is_ok() {
        // Also strip any %OMNI-OWNER-LOCKED or %OMNI-USER-PW comment markers if present
        if let Ok(saved_bytes) = std::fs::read(out_path) {
            if saved_bytes.windows(10).any(|w| w == b"%OMNI-USER" || w == b"%OMNI-OWNE") {
                let cleaned = strip_omni_markers(&saved_bytes);
                let _ = std::fs::write(out_path, cleaned);
            }
        }
        return Ok(());
    }
    let cleaned = strip_omni_markers(raw_bytes);
    std::fs::write(out_path, cleaned).map_err(|e| e.to_string())
}

fn strip_omni_markers(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let filtered: Vec<&str> = text
        .lines()
        .filter(|l| {
            !l.starts_with("%OMNI-OWNER-LOCKED")
                && !l.starts_with("%OMNI-USER-PW:")
                && !l.contains("/Encrypt")
        })
        .collect();
    filtered.join("\n").into_bytes()
}

fn unlock_archive_file(
    file_id: &str,
    input_path: &Path,
    format: &str,
    output_dir: &str,
    manual_password: Option<&str>,
    vault_entries: &[VaultEntry],
) -> Result<UnlockResult, String> {
    let out_dir = ensure_output_dir(output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    let out_path = out_dir.join(format!("{}_unlocked.{}", stem, format));

    // Build candidate list: manual password first, then all Vault entries
    let mut candidates: Vec<(String, String, Option<String>)> = Vec::new();
    if let Some(pw) = manual_password.filter(|s| !s.is_empty()) {
        candidates.push((pw.to_string(), "manual_password".to_string(), None));
    }
    for v in vault_entries {
        candidates.push((
            v.password.clone(),
            "auto_vault".to_string(),
            Some(v.label.clone()),
        ));
    }

    for (pw, method, label) in candidates {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        if extract_archive_to_dir(input_path, format, tmp.path(), Some(&pw)).is_ok() {
            pack_dir_to_archive(tmp.path(), &out_path, format)?;
            let new_status = if method == "auto_vault" {
                EncryptionStatus::AutoUnlocked
            } else {
                EncryptionStatus::ManuallyUnlocked
            };
            return Ok(UnlockResult {
                file_id: file_id.to_string(),
                input_path: input_path.to_string_lossy().to_string(),
                success: true,
                output_path: Some(out_path.to_string_lossy().to_string()),
                new_status,
                method_used: method,
                matched_label: label.clone(),
                message: if let Some(lbl) = label {
                    format!(
                        "Archívum automatikusan feloldva a Saját Jelszótárból ({}) és jelszómentesen elmentve!",
                        lbl
                    )
                } else {
                    "Archívum sikeresen feloldva és jelszómentesen elmentve!".to_string()
                },
            });
        }
    }

    Ok(UnlockResult {
        file_id: file_id.to_string(),
        input_path: input_path.to_string_lossy().to_string(),
        success: false,
        output_path: None,
        new_status: EncryptionStatus::UserLocked,
        method_used: "manual_password".to_string(),
        matched_label: None,
        message: "Nem sikerült feloldani az archívumot a megadott jelszóval vagy a Saját Jelszótárral."
            .to_string(),
    })
}

fn vault_storage_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let dir = Path::new(&home).join(".config").join("omniconvert");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("vault.json")
    } else {
        PathBuf::from(".omniconvert-vault.json")
    }
}

pub fn load_vault() -> Vec<VaultEntry> {
    let p = vault_storage_path();
    if let Ok(content) = std::fs::read_to_string(&p) {
        if let Ok(entries) = serde_json::from_str::<Vec<VaultEntry>>(&content) {
            if !entries.is_empty() {
                return entries;
            }
        }
    }
    vec![
        VaultEntry {
            id: "vault-default-1".to_string(),
            label: "Céges Számla / Adószám Kód".to_string(),
            password: "12345678".to_string(),
            category_hint: "Számla PDF / NAV".to_string(),
            created_at: "2026-11-01".to_string(),
        },
        VaultEntry {
            id: "vault-default-2".to_string(),
            label: "Banki Kivonat Alapjelszó".to_string(),
            password: "omniconvert2026".to_string(),
            category_hint: "Banki PDF / ZIP".to_string(),
            created_at: "2026-11-01".to_string(),
        },
    ]
}

pub fn save_vault(entries: &[VaultEntry]) -> Result<Vec<VaultEntry>, String> {
    let p = vault_storage_path();
    let json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    std::fs::write(&p, json).map_err(|e| format!("Jelszótár mentési hiba: {}", e))?;
    Ok(entries.to_vec())
}
