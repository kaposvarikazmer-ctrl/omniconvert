use crate::models::{ConversionOptions, PdfToolkitResult, SingleConversionResponse};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

pub fn convert_image_file(
    input_path: &Path,
    target_format: &str,
    options: &ConversionOptions,
) -> Result<SingleConversionResponse, String> {
    let original_size = std::fs::metadata(input_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let ext_lower = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Load image (either raster or SVG vector rasterization via resvg)
    let mut img = if ext_lower == "svg" || is_svg_file(input_path) {
        rasterize_svg(input_path, options.resize_width, options.resize_height)?
    } else {
        ImageReader::open(input_path)
            .map_err(|e| format!("Kép megnyitási hiba: {}", e))?
            .with_guessed_format()
            .map_err(|e| format!("Képformátum felismerési hiba: {}", e))?
            .decode()
            .map_err(|e| format!("Kép dekódolási hiba: {}", e))?
    };

    // Apply optional resizing
    if options.resize_width.is_some() || options.resize_height.is_some() {
        let (orig_w, orig_h) = img.dimensions();
        let target_w = options.resize_width.unwrap_or(orig_w).max(1);
        let target_h = options.resize_height.unwrap_or(orig_h).max(1);
        img = if options.keep_aspect_ratio {
            img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3)
        } else {
            img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3)
        };
    }

    let out_dir = ensure_output_dir(&options.output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted");
    let target_clean = target_format.trim_start_matches('.').to_lowercase();
    let out_path = out_dir.join(format!("{}.{}", stem, target_clean));

    if target_clean == "pdf" {
        let res = merge_images_into_pdf(
            &[input_path.to_string_lossy().to_string()],
            &out_dir.to_string_lossy(),
            &format!("{}.pdf", stem),
            options.image_quality,
        )?;
        let final_path = res
            .output_paths
            .first()
            .cloned()
            .unwrap_or_else(|| out_path.to_string_lossy().to_string());
        return Ok(SingleConversionResponse {
            output_path: final_path,
            output_size_bytes: res.output_size_bytes,
            savings_percent: res.savings_percent,
        });
    }

    // Re-encoding from DynamicImage automatically strips raw EXIF/GPS metadata segments
    match target_clean.as_str() {
        "jpg" | "jpeg" => {
            let file = File::create(&out_path).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);
            let quality = options.image_quality.clamp(1, 100);
            let rgb = img.to_rgb8();
            let mut encoder = JpegEncoder::new_with_quality(&mut writer, quality);
            encoder
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| format!("JPEG kódolási hiba: {}", e))?;
        }
        "png" => {
            img.save_with_format(&out_path, ImageFormat::Png)
                .map_err(|e| format!("PNG mentési hiba: {}", e))?;
        }
        "webp" => {
            img.save_with_format(&out_path, ImageFormat::WebP)
                .map_err(|e| format!("WEBP mentési hiba: {}", e))?;
        }
        "avif" => {
            img.save_with_format(&out_path, ImageFormat::Avif)
                .map_err(|e| format!("AVIF mentési hiba: {}", e))?;
        }
        "gif" => {
            img.save_with_format(&out_path, ImageFormat::Gif)
                .map_err(|e| format!("GIF mentési hiba: {}", e))?;
        }
        "tiff" | "tif" => {
            img.save_with_format(&out_path, ImageFormat::Tiff)
                .map_err(|e| format!("TIFF mentési hiba: {}", e))?;
        }
        "bmp" => {
            img.save_with_format(&out_path, ImageFormat::Bmp)
                .map_err(|e| format!("BMP mentési hiba: {}", e))?;
        }
        "ico" => {
            // ICO supports max 256x256
            let (w, h) = img.dimensions();
            let ico_img = if w > 256 || h > 256 {
                img.resize(256, 256, image::imageops::FilterType::Lanczos3)
            } else {
                img
            };
            ico_img
                .save_with_format(&out_path, ImageFormat::Ico)
                .map_err(|e| format!("ICO mentési hiba: {}", e))?;
        }
        "tga" => {
            img.save_with_format(&out_path, ImageFormat::Tga)
                .map_err(|e| format!("TGA mentési hiba: {}", e))?;
        }
        "pnm" | "ppm" => {
            img.save_with_format(&out_path, ImageFormat::Pnm)
                .map_err(|e| format!("PNM mentési hiba: {}", e))?;
        }
        "qoi" => {
            img.save_with_format(&out_path, ImageFormat::Qoi)
                .map_err(|e| format!("QOI mentési hiba: {}", e))?;
        }
        _ => {
            img.save(&out_path)
                .map_err(|e| format!("Kép mentési hiba ({}): {}", target_clean, e))?;
        }
    }

    let output_size = std::fs::metadata(&out_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let savings = calculate_savings(original_size, output_size);

    Ok(SingleConversionResponse {
        output_path: out_path.to_string_lossy().to_string(),
        output_size_bytes: output_size,
        savings_percent: savings,
    })
}

fn is_svg_file(path: &Path) -> bool {
    if let Ok(bytes) = std::fs::read(path) {
        let prefix = &bytes[..bytes.len().min(512)];
        if let Ok(s) = std::str::from_utf8(prefix) {
            return s.contains("<svg");
        }
    }
    false
}

pub fn rasterize_svg(
    path: &Path,
    target_w: Option<u32>,
    target_h: Option<u32>,
) -> Result<DynamicImage, String> {
    let svg_data = std::fs::read(path).map_err(|e| format!("SVG olvasási hiba: {}", e))?;
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&svg_data, &opt)
        .map_err(|e| format!("SVG elemzési hiba: {}", e))?;

    let orig_size = tree.size();
    let w = target_w.unwrap_or((orig_size.width() as u32).max(512));
    let h = target_h.unwrap_or((orig_size.height() as u32).max(512));

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| "Nem sikerült létrehozni a raszteres vásznat".to_string())?;

    let sx = w as f32 / orig_size.width().max(1.0);
    let sy = h as f32 / orig_size.height().max(1.0);
    let transform = resvg::tiny_skia::Transform::from_scale(sx, sy);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let rgba = image::RgbaImage::from_raw(w, h, pixmap.data().to_vec())
        .ok_or_else(|| "RGBA konverziós hiba".to_string())?;
    Ok(DynamicImage::ImageRgba8(rgba))
}

/// Merges one or more images into a single multi-page PDF document using lopdf.
pub fn merge_images_into_pdf(
    input_paths: &[String],
    output_dir: &str,
    output_file_name: &str,
    quality: u8,
) -> Result<PdfToolkitResult, String> {
    if input_paths.is_empty() {
        return Err("Nincs kiválasztott képfájl a PDF-be fűzéshez.".to_string());
    }

    let first_path = Path::new(&input_paths[0]);
    let out_dir = ensure_output_dir(output_dir, first_path)?;
    let file_name = if output_file_name.ends_with(".pdf") {
        output_file_name.to_string()
    } else {
        format!("{}.pdf", output_file_name)
    };
    let out_path = out_dir.join(file_name);

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut page_ids = Vec::new();
    let mut total_input_bytes = 0u64;

    for path_str in input_paths {
        let p = Path::new(path_str);
        total_input_bytes += std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);

        let img = if p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("svg"))
            .unwrap_or(false)
            || is_svg_file(p)
        {
            rasterize_svg(p, None, None)?
        } else {
            ImageReader::open(p)
                .map_err(|e| format!("Kép megnyitási hiba ({}): {}", path_str, e))?
                .with_guessed_format()
                .map_err(|e| e.to_string())?
                .decode()
                .map_err(|e| format!("Kép dekódolási hiba ({}): {}", path_str, e))?
        };

        let rgb = img.to_rgb8();
        let (w, h) = (rgb.width(), rgb.height());

        let mut jpeg_bytes = Vec::new();
        {
            let mut enc = JpegEncoder::new_with_quality(&mut jpeg_bytes, quality.clamp(30, 95));
            enc.encode(
                rgb.as_raw(),
                w,
                h,
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| e.to_string())?;
        }

        let img_stream = Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => w as i64,
                "Height" => h as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            jpeg_bytes,
        );
        let img_id = doc.add_object(img_stream);

        // Fit image onto A4 portrait/landscape or exact pixel size in points
        let page_w = (w as f64).clamp(200.0, 842.0);
        let page_h = (h as f64 * (page_w / w.max(1) as f64)).clamp(200.0, 1190.0);

        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        page_w.into(),
                        0.into(),
                        0.into(),
                        page_h.into(),
                        0.into(),
                        0.into(),
                    ],
                ),
                Operation::new("Do", vec![Object::Name(b"Im0".to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap_or_default()));

        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! {
                "Im0" => img_id,
            },
        });

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), page_w.into(), page_h.into()],
            "Contents" => content_id,
            "Resources" => resources_id,
        });
        page_ids.push(page_id.into());
    }

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids,
            "Count" => input_paths.len() as i64,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    doc.save(&out_path)
        .map_err(|e| format!("PDF mentési hiba: {}", e))?;

    let out_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    let savings = calculate_savings(total_input_bytes, out_size);

    Ok(PdfToolkitResult {
        success: true,
        output_paths: vec![out_path.to_string_lossy().to_string()],
        original_size_bytes: total_input_bytes,
        output_size_bytes: out_size,
        savings_percent: savings,
        message: format!("{} kép sikeresen egyesítve PDF dokumentummá.", input_paths.len()),
    })
}

pub fn ensure_output_dir(output_dir: &str, fallback_file: &Path) -> Result<PathBuf, String> {
    let dir_path = if output_dir.trim().is_empty() || output_dir == "./omniconvert-output" {
        fallback_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("omniconvert-output")
    } else {
        PathBuf::from(output_dir)
    };
    std::fs::create_dir_all(&dir_path)
        .map_err(|e| format!("Kimeneti mappa létrehozási hiba: {}", e))?;
    Ok(dir_path)
}

pub fn calculate_savings(orig: u64, out: u64) -> f64 {
    if orig == 0 || out >= orig {
        0.0
    } else {
        (((orig - out) as f64 / orig as f64) * 1000.0).round() / 10.0
    }
}
