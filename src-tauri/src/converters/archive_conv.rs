use crate::converters::image_conv::{calculate_savings, ensure_output_dir};
use crate::models::{ConversionOptions, SingleConversionResponse};
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use bzip2::Compression as BzCompression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression as GzCompression;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

pub fn convert_archive_file(
    input_path: &Path,
    source_format: &str,
    target_format: &str,
    options: &ConversionOptions,
) -> Result<SingleConversionResponse, String> {
    let original_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let temp_dir =
        tempfile::tempdir().map_err(|e| format!("Ideiglenes mappa létrehozási hiba: {}", e))?;

    extract_archive_to_dir(input_path, source_format, temp_dir.path(), None)?;

    let out_dir = ensure_output_dir(&options.output_dir, input_path)?;
    let raw_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    let stem = raw_stem.trim_end_matches(".tar");
    let target_clean = target_format.trim_start_matches('.').to_lowercase();
    let out_path = out_dir.join(format!("{}.{}", stem, target_clean));

    pack_dir_to_archive(temp_dir.path(), &out_path, &target_clean)?;

    let output_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    Ok(SingleConversionResponse {
        output_path: out_path.to_string_lossy().to_string(),
        output_size_bytes: output_size,
        savings_percent: calculate_savings(original_size, output_size),
    })
}

pub fn extract_archive_to_dir(
    input_path: &Path,
    source_format: &str,
    dest_dir: &Path,
    password: Option<&str>,
) -> Result<(), String> {
    let fmt = source_format.to_lowercase();
    match fmt.as_str() {
        "zip" => {
            let file = File::open(input_path).map_err(|e| e.to_string())?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| format!("ZIP megnyitási hiba: {}", e))?;

            for i in 0..archive.len() {
                if let Some(pw) = password {
                    let mut entry = archive
                        .by_index_decrypt(i, pw.as_bytes())
                        .map_err(|e| format!("ZIP dekódolási hiba (hibás jelszó?): {}", e))?;
                    write_zip_entry(&mut entry, dest_dir)?;
                } else {
                    let mut entry = archive
                        .by_index(i)
                        .map_err(|e| format!("ZIP elem olvasási hiba: {}", e))?;
                    write_zip_entry(&mut entry, dest_dir)?;
                }
            }
            Ok(())
        }
        "7z" => {
            if let Some(pw) = password {
                sevenz_rust::decompress_file_with_password(
                    input_path,
                    dest_dir,
                    sevenz_rust::Password::from(pw),
                )
                .map_err(|e| format!("7Z kicsomagolási hiba (jelszóval): {}", e))
            } else {
                sevenz_rust::decompress_file(input_path, dest_dir)
                    .map_err(|e| format!("7Z kicsomagolási hiba: {}", e))
            }
        }
        "tar" => {
            let file = File::open(input_path).map_err(|e| e.to_string())?;
            let mut archive = tar::Archive::new(file);
            archive
                .unpack(dest_dir)
                .map_err(|e| format!("TAR kicsomagolási hiba: {}", e))
        }
        "tar.gz" | "tgz" => {
            let file = File::open(input_path).map_err(|e| e.to_string())?;
            let gz = GzDecoder::new(file);
            let mut archive = tar::Archive::new(gz);
            archive
                .unpack(dest_dir)
                .map_err(|e| format!("TAR.GZ kicsomagolási hiba: {}", e))
        }
        "tar.bz2" | "tbz2" => {
            let file = File::open(input_path).map_err(|e| e.to_string())?;
            let bz = BzDecoder::new(file);
            let mut archive = tar::Archive::new(bz);
            archive
                .unpack(dest_dir)
                .map_err(|e| format!("TAR.BZ2 kicsomagolási hiba: {}", e))
        }
        "tar.xz" | "txz" => {
            let file = File::open(input_path).map_err(|e| e.to_string())?;
            let mut reader = BufReader::new(file);
            let mut decompressed = Vec::new();
            lzma_rs::xz_decompress(&mut reader, &mut decompressed)
                .map_err(|e| format!("TAR.XZ kicsomagolási hiba: {}", e))?;
            let mut archive = tar::Archive::new(std::io::Cursor::new(decompressed));
            archive
                .unpack(dest_dir)
                .map_err(|e| format!("TAR.XZ kicsomagolási hiba: {}", e))
        }
        _ => {
            // Fallback: copy single file into dest_dir
            let fname = input_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("payload.bin");
            std::fs::copy(input_path, dest_dir.join(fname))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    }
}

fn write_zip_entry(
    entry: &mut zip::read::ZipFile<'_>,
    dest_dir: &Path,
) -> Result<(), String> {
    let enclosed = entry
        .enclosed_name()
        .ok_or_else(|| "Érvénytelen fájlnév a ZIP archívumban".to_string())?;
    let out_path = dest_dir.join(enclosed);
    if entry.is_dir() {
        std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
    } else {
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out_file = File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(entry, &mut out_file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn pack_dir_to_archive(
    src_dir: &Path,
    out_path: &Path,
    target_format: &str,
) -> Result<(), String> {
    match target_format {
        "zip" => {
            let file = File::create(out_path).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipWriter::new(file);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o644);

            for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                let rel = path.strip_prefix(src_dir).map_err(|e| e.to_string())?;
                if rel.as_os_str().is_empty() {
                    continue;
                }
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if path.is_dir() {
                    zip.add_directory(&rel_str, options)
                        .map_err(|e| e.to_string())?;
                } else {
                    zip.start_file(&rel_str, options)
                        .map_err(|e| e.to_string())?;
                    let mut f = File::open(path).map_err(|e| e.to_string())?;
                    std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
                }
            }
            zip.finish().map_err(|e| e.to_string())?;
            Ok(())
        }
        "7z" => sevenz_rust::compress_to_path(src_dir, out_path)
            .map_err(|e| format!("7Z tömörítési hiba: {}", e)),
        "tar" => {
            let file = File::create(out_path).map_err(|e| e.to_string())?;
            let mut builder = tar::Builder::new(file);
            builder
                .append_dir_all(".", src_dir)
                .map_err(|e| e.to_string())?;
            builder.finish().map_err(|e| e.to_string())
        }
        "tar.gz" | "tgz" => {
            let file = File::create(out_path).map_err(|e| e.to_string())?;
            let enc = GzEncoder::new(file, GzCompression::default());
            let mut builder = tar::Builder::new(enc);
            builder
                .append_dir_all(".", src_dir)
                .map_err(|e| e.to_string())?;
            builder.finish().map_err(|e| e.to_string())
        }
        "tar.bz2" | "tbz2" => {
            let file = File::create(out_path).map_err(|e| e.to_string())?;
            let enc = BzEncoder::new(file, BzCompression::default());
            let mut builder = tar::Builder::new(enc);
            builder
                .append_dir_all(".", src_dir)
                .map_err(|e| e.to_string())?;
            builder.finish().map_err(|e| e.to_string())
        }
        "tar.xz" | "txz" => {
            let mut tar_bytes = Vec::new();
            {
                let mut builder = tar::Builder::new(&mut tar_bytes);
                builder
                    .append_dir_all(".", src_dir)
                    .map_err(|e| e.to_string())?;
                builder.finish().map_err(|e| e.to_string())?;
            }
            let file = File::create(out_path).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);
            let mut cursor = std::io::Cursor::new(tar_bytes);
            lzma_rs::xz_compress(&mut cursor, &mut writer)
                .map_err(|e| format!("XZ tömörítési hiba: {}", e))?;
            writer.flush().map_err(|e| e.to_string())
        }
        _ => Err(format!("Nem támogatott archívum célformátum: {}", target_format)),
    }
}
