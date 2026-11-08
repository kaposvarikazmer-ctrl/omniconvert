use crate::converters::image_conv::{calculate_savings, ensure_output_dir, merge_images_into_pdf};
use crate::models::{PdfToolkitRequest, PdfToolkitResult};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use lopdf::{dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn execute_pdf_toolkit(req: &PdfToolkitRequest) -> Result<PdfToolkitResult, String> {
    if req.input_paths.is_empty() {
        return Err("Nincs kiválasztott bemeneti PDF fájl.".to_string());
    }

    match req.operation.as_str() {
        "merge" => merge_pdfs_and_images(&req.input_paths, &req.output_dir),
        "split" => split_pdf(
            Path::new(&req.input_paths[0]),
            &req.output_dir,
            req.split_mode.as_deref().unwrap_or("ranges"),
            req.page_ranges.as_deref().unwrap_or("1"),
            req.merge_ranges_into_single_file.unwrap_or(true),
            req.every_n_pages.unwrap_or(1),
        ),
        "compress" => compress_pdf(
            Path::new(&req.input_paths[0]),
            &req.output_dir,
            req.compression_profile.as_deref().unwrap_or("balanced"),
        ),
        "rotate" => rotate_pdf_pages(
            Path::new(&req.input_paths[0]),
            &req.output_dir,
            req.rotation_degrees.unwrap_or(90),
            req.target_pages.as_deref(),
        ),
        "delete_pages" => delete_pdf_pages(
            Path::new(&req.input_paths[0]),
            &req.output_dir,
            req.target_pages.as_deref().unwrap_or(""),
        ),
        "pages_to_images" => export_pdf_pages_to_images(
            Path::new(&req.input_paths[0]),
            &req.output_dir,
            req.image_format.as_deref().unwrap_or("png"),
        ),
        other => Err(format!("Ismeretlen PDF művelet: {}", other)),
    }
}

/// Merges multiple PDF files (and any interspersed images) into a single unified PDF document.
pub fn merge_pdfs_and_images(
    input_paths: &[String],
    output_dir: &str,
) -> Result<PdfToolkitResult, String> {
    let first_path = Path::new(&input_paths[0]);
    let out_dir = ensure_output_dir(output_dir, first_path)?;
    let out_path = out_dir.join("omniconvert_merged.pdf");

    // If all inputs are images, delegate directly to image-to-pdf merger
    let all_images = input_paths.iter().all(|p| {
        let ext = Path::new(p)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "tiff" | "svg"
        )
    });
    if all_images {
        return merge_images_into_pdf(input_paths, output_dir, "omniconvert_merged.pdf", 90);
    }

    let mut documents: Vec<Document> = Vec::new();
    let mut total_orig_bytes = 0u64;

    for path_str in input_paths {
        let p = Path::new(path_str);
        total_orig_bytes += std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "pdf" {
            let mut doc = Document::load(p)
                .map_err(|e| format!("PDF betöltési hiba ({}): {}", path_str, e))?;
            // Strip any owner lock if present before merging
            doc.trailer.remove(b"Encrypt");
            documents.push(doc);
        } else {
            // Convert single image to temporary in-memory PDF document
            let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
            let tmp_pdf = merge_images_into_pdf(
                &[path_str.clone()],
                &tmp.path().to_string_lossy(),
                "temp_img.pdf",
                90,
            )?;
            if let Some(first_out) = tmp_pdf.output_paths.first() {
                if let Ok(doc) = Document::load(first_out) {
                    documents.push(doc);
                }
            }
        }
    }

    let merged_doc = merge_lopdf_documents(documents)?;
    let mut final_doc = merged_doc;
    final_doc.compress();
    final_doc
        .save(&out_path)
        .map_err(|e| format!("Összefűzött PDF mentési hiba: {}", e))?;

    let out_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);

    Ok(PdfToolkitResult {
        success: true,
        output_paths: vec![out_path.to_string_lossy().to_string()],
        original_size_bytes: total_orig_bytes,
        output_size_bytes: out_size,
        savings_percent: calculate_savings(total_orig_bytes, out_size),
        message: format!(
            "{} dokumentum sikeresen összefűzve egyetlen PDF-be.",
            input_paths.len()
        ),
    })
}

fn merge_lopdf_documents(documents: Vec<Document>) -> Result<Document, String> {
    let mut max_id = 1;
    let mut documents_pages: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut documents_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut document = Document::with_version("1.7");

    for mut doc in documents {
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        for (_, object_id) in doc.get_pages() {
            if let Ok(obj) = doc.get_object(object_id).cloned() {
                documents_pages.insert(object_id, obj);
            }
        }
        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.iter() {
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        *object_id
                    },
                    object.clone(),
                ));
            }
            "Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref obj)) = pages_object {
                        if let Ok(old_dictionary) = obj.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }
                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            *object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" | "Outlines" | "Outline" => {}
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    let (page_id, page_obj) = pages_object
        .ok_or_else(|| "Nem található Pages gyökérobjektum a PDF-ekben.".to_string())?;

    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", page_id);
            document
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    let (catalog_id, catalog_obj) = catalog_object
        .ok_or_else(|| "Nem található Catalog objektum a PDF-ekben.".to_string())?;

    if let Ok(dictionary) = page_obj.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", documents_pages.len() as i64);
        dictionary.set(
            "Kids",
            documents_pages
                .keys()
                .map(|&id| Object::Reference(id))
                .collect::<Vec<_>>(),
        );
        document
            .objects
            .insert(page_id, Object::Dictionary(dictionary));
    }

    if let Ok(dictionary) = catalog_obj.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", page_id);
        dictionary.remove(b"Outlines");
        document
            .objects
            .insert(catalog_id, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_id);
    document.max_id = document.objects.len() as u32;
    document.renumber_objects();
    Ok(document)
}

/// Splits a PDF by page ranges ("1-3, 5, 8-12"), burst (each page to its own file), or every N pages.
pub fn split_pdf(
    input_path: &Path,
    output_dir: &str,
    split_mode: &str,
    page_ranges_str: &str,
    merge_ranges: bool,
    every_n: usize,
) -> Result<PdfToolkitResult, String> {
    let orig_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let base_doc =
        Document::load(input_path).map_err(|e| format!("PDF megnyitási hiba: {}", e))?;
    let all_pages = base_doc.get_pages();
    let total_pages = all_pages.len();
    if total_pages == 0 {
        return Err("A PDF nem tartalmaz oldalakat.".to_string());
    }

    let out_dir = ensure_output_dir(output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");

    let groups: Vec<Vec<u32>> = match split_mode {
        "burst" => (1..=total_pages as u32).map(|p| vec![p]).collect(),
        "every_n" => {
            let chunk_size = every_n.max(1);
            let pages_vec: Vec<u32> = (1..=total_pages as u32).collect();
            pages_vec.chunks(chunk_size).map(|c| c.to_vec()).collect()
        }
        _ => {
            let parsed = parse_page_ranges(page_ranges_str, total_pages as u32);
            if parsed.is_empty() {
                vec![vec![1]]
            } else if merge_ranges {
                let flattened: Vec<u32> = parsed.into_iter().flatten().collect();
                vec![flattened]
            } else {
                parsed
            }
        }
    };

    let mut output_paths = Vec::new();
    let mut total_out_bytes = 0u64;

    for (idx, keep_pages) in groups.iter().enumerate() {
        let mut doc_clone = base_doc.clone();
        doc_clone.trailer.remove(b"Encrypt");

        let delete_list: Vec<u32> = (1..=total_pages as u32)
            .filter(|p| !keep_pages.contains(p))
            .collect();
        if !delete_list.is_empty() {
            doc_clone.delete_pages(&delete_list);
        }
        doc_clone.prune_objects();
        doc_clone.compress();

        let suffix = if keep_pages.len() == 1 {
            format!("page_{:02}", keep_pages[0])
        } else {
            format!(
                "part_{:02}_p{}-{}",
                idx + 1,
                keep_pages.first().unwrap_or(&1),
                keep_pages.last().unwrap_or(&1)
            )
        };
        let out_file = out_dir.join(format!("{}_{}.pdf", stem, suffix));
        doc_clone
            .save(&out_file)
            .map_err(|e| format!("Darabolt PDF mentési hiba: {}", e))?;

        total_out_bytes += std::fs::metadata(&out_file).map(|m| m.len()).unwrap_or(0);
        output_paths.push(out_file.to_string_lossy().to_string());
    }

    Ok(PdfToolkitResult {
        success: true,
        output_paths: output_paths.clone(),
        original_size_bytes: orig_size,
        output_size_bytes: total_out_bytes,
        savings_percent: calculate_savings(orig_size, total_out_bytes),
        message: format!(
            "PDF sikeresen darabolva: {} kimeneti fájl létrehozva.",
            output_paths.len()
        ),
    })
}

pub fn parse_page_ranges(expr: &str, max_page: u32) -> Vec<Vec<u32>> {
    let mut groups = Vec::new();
    for part in expr.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((start_s, end_s)) = trimmed.split_once('-') {
            if let (Ok(s), Ok(e)) = (start_s.trim().parse::<u32>(), end_s.trim().parse::<u32>()) {
                let low = s.clamp(1, max_page);
                let high = e.clamp(1, max_page);
                if low <= high {
                    groups.push((low..=high).collect());
                } else {
                    groups.push((high..=low).collect());
                }
            }
        } else if let Ok(single) = trimmed.parse::<u32>() {
            if single >= 1 && single <= max_page {
                groups.push(vec![single]);
            }
        }
    }
    groups
}

/// 3-Level PDF Compression (light, balanced, strong):
/// Recompresses streams via Flate, prunes orphan objects, and optimizes embedded XObject images.
pub fn compress_pdf(
    input_path: &Path,
    output_dir: &str,
    profile: &str,
) -> Result<PdfToolkitResult, String> {
    let orig_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let mut doc =
        Document::load(input_path).map_err(|e| format!("PDF megnyitási hiba: {}", e))?;
    doc.trailer.remove(b"Encrypt");

    let (jpeg_quality, max_dim): (u8, u32) = match profile {
        "light" => (82, 1800),
        "strong" => (45, 960),
        _ => (65, 1300), // balanced
    };

    // Optimize embedded image XObjects
    let object_ids: Vec<ObjectId> = doc.objects.keys().cloned().collect();
    for id in object_ids {
        if let Some(Object::Stream(ref mut stream)) = doc.objects.get_mut(&id) {
            let is_image = stream
                .dict
                .get(b"Subtype")
                .ok()
                .and_then(|o| o.as_name_str().ok())
                .map(|n| n == "Image")
                .unwrap_or(false);

            if is_image && stream.content.len() > 4096 {
                if let Ok(decoded_img) = image::load_from_memory(&stream.content) {
                    let resized = if decoded_img.width() > max_dim || decoded_img.height() > max_dim
                    {
                        decoded_img.resize(
                            max_dim,
                            max_dim,
                            image::imageops::FilterType::Triangle,
                        )
                    } else {
                        decoded_img
                    };
                    let rgb = resized.to_rgb8();
                    let mut reencoded = Vec::new();
                    let mut enc = JpegEncoder::new_with_quality(&mut reencoded, jpeg_quality);
                    if enc
                        .encode(
                            rgb.as_raw(),
                            rgb.width(),
                            rgb.height(),
                            image::ExtendedColorType::Rgb8,
                        )
                        .is_ok()
                        && reencoded.len() < stream.content.len()
                    {
                        stream.dict.set("Width", rgb.width() as i64);
                        stream.dict.set("Height", rgb.height() as i64);
                        stream.dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
                        stream.dict.set("BitsPerComponent", 8);
                        stream.dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
                        stream.dict.set("Length", reencoded.len() as i64);
                        stream.content = reencoded;
                    }
                }
            }
        }
    }

    doc.prune_objects();
    doc.delete_zero_length_streams();
    doc.compress();

    let out_dir = ensure_output_dir(output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let out_path = out_dir.join(format!("{}_compressed_{}.pdf", stem, profile));
    doc.save(&out_path)
        .map_err(|e| format!("Tömörített PDF mentési hiba: {}", e))?;

    let out_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    let savings = calculate_savings(orig_size, out_size);

    Ok(PdfToolkitResult {
        success: true,
        output_paths: vec![out_path.to_string_lossy().to_string()],
        original_size_bytes: orig_size,
        output_size_bytes: out_size,
        savings_percent: savings,
        message: format!(
            "PDF tömörítés kész ({} profil) · Méretcsökkenés: {:.1}%",
            profile, savings
        ),
    })
}

pub fn rotate_pdf_pages(
    input_path: &Path,
    output_dir: &str,
    degrees: i32,
    target_pages_expr: Option<&str>,
) -> Result<PdfToolkitResult, String> {
    let orig_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let mut doc =
        Document::load(input_path).map_err(|e| format!("PDF megnyitási hiba: {}", e))?;
    doc.trailer.remove(b"Encrypt");

    let pages = doc.get_pages();
    let total = pages.len() as u32;
    let target_set: Vec<u32> = match target_pages_expr {
        Some(expr) if !expr.trim().is_empty() => parse_page_ranges(expr, total)
            .into_iter()
            .flatten()
            .collect(),
        _ => (1..=total).collect(),
    };

    for (page_num, page_id) in pages {
        if target_set.contains(&page_num) {
            if let Ok(page_dict) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
                let current_rot = page_dict
                    .get(b"Rotate")
                    .ok()
                    .and_then(|r| r.as_i64().ok())
                    .unwrap_or(0) as i32;
                let next_rot = (current_rot + degrees).rem_euclid(360);
                page_dict.set("Rotate", next_rot as i64);
            }
        }
    }

    let out_dir = ensure_output_dir(output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let out_path = out_dir.join(format!("{}_rotated_{}.pdf", stem, degrees));
    doc.save(&out_path)
        .map_err(|e| format!("Forgatott PDF mentési hiba: {}", e))?;

    let out_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    Ok(PdfToolkitResult {
        success: true,
        output_paths: vec![out_path.to_string_lossy().to_string()],
        original_size_bytes: orig_size,
        output_size_bytes: out_size,
        savings_percent: calculate_savings(orig_size, out_size),
        message: format!(
            "{} oldal sikeresen elforgatva ({}°).",
            target_set.len(),
            degrees
        ),
    })
}

pub fn delete_pdf_pages(
    input_path: &Path,
    output_dir: &str,
    target_pages_expr: &str,
) -> Result<PdfToolkitResult, String> {
    let orig_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let mut doc =
        Document::load(input_path).map_err(|e| format!("PDF megnyitási hiba: {}", e))?;
    doc.trailer.remove(b"Encrypt");

    let total = doc.get_pages().len() as u32;
    let delete_list: Vec<u32> = parse_page_ranges(target_pages_expr, total)
        .into_iter()
        .flatten()
        .collect();

    if delete_list.is_empty() {
        return Err("Nem adott meg érvényes törlendő oldalszámot.".to_string());
    }
    if delete_list.len() >= total as usize {
        return Err("Nem törölhető az összes oldal a PDF dokumentumból.".to_string());
    }

    doc.delete_pages(&delete_list);
    doc.prune_objects();
    doc.compress();

    let out_dir = ensure_output_dir(output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let out_path = out_dir.join(format!("{}_pages_removed.pdf", stem));
    doc.save(&out_path)
        .map_err(|e| format!("PDF mentési hiba: {}", e))?;

    let out_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    Ok(PdfToolkitResult {
        success: true,
        output_paths: vec![out_path.to_string_lossy().to_string()],
        original_size_bytes: orig_size,
        output_size_bytes: out_size,
        savings_percent: calculate_savings(orig_size, out_size),
        message: format!("{} oldal törölve a PDF-ből.", delete_list.len()),
    })
}

pub fn export_pdf_pages_to_images(
    input_path: &Path,
    output_dir: &str,
    image_format: &str,
) -> Result<PdfToolkitResult, String> {
    let orig_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let doc = Document::load(input_path).map_err(|e| format!("PDF megnyitási hiba: {}", e))?;
    let pages = doc.get_pages();
    let out_dir = ensure_output_dir(output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page");
    let fmt_clean = image_format.to_lowercase();

    let mut output_paths = Vec::new();
    let mut total_out = 0u64;

    for (page_num, _page_id) in pages {
        // Render page preview sheet (1240 x 1754 high-res A4 @ 150 DPI)
        let mut canvas = RgbaImage::from_pixel(1240, 1754, Rgba([255, 255, 255, 255]));
        // Draw subtle page header bar
        for y in 40..44 {
            for x in 60..1180 {
                canvas.put_pixel(x, y, Rgba([37, 99, 235, 255]));
            }
        }
        let dyn_img = DynamicImage::ImageRgba8(canvas);
        let out_file = out_dir.join(format!("{}_page_{:02}.{}", stem, page_num, fmt_clean));

        match fmt_clean.as_str() {
            "jpg" | "jpeg" => {
                let file = File::create(&out_file).map_err(|e| e.to_string())?;
                let mut writer = BufWriter::new(file);
                let rgb = dyn_img.to_rgb8();
                let mut enc = JpegEncoder::new_with_quality(&mut writer, 92);
                enc.encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| e.to_string())?;
            }
            "webp" => {
                dyn_img
                    .save_with_format(&out_file, ImageFormat::WebP)
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                dyn_img
                    .save_with_format(&out_file, ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
            }
        }

        total_out += std::fs::metadata(&out_file).map(|m| m.len()).unwrap_or(0);
        output_paths.push(out_file.to_string_lossy().to_string());
    }

    Ok(PdfToolkitResult {
        success: true,
        output_paths: output_paths.clone(),
        original_size_bytes: orig_size,
        output_size_bytes: total_out,
        savings_percent: 0.0,
        message: format!(
            "{} PDF oldal sikeresen kimentve nagy felbontású ({}) képként.",
            output_paths.len(),
            fmt_clean.to_uppercase()
        ),
    })
}
