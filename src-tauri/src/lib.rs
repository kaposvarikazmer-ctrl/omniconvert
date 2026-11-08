pub mod converters;
pub mod deps;
pub mod inspector;
pub mod models;
pub mod pdf_toolkit;
pub mod unlocker;

use base64::Engine;
use image::{Rgba, RgbaImage};
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use models::{
    ConversionOptions, DependencyStatus, InspectedFile, PdfToolkitRequest, PdfToolkitResult,
    SingleConversionResponse, UnlockResult, VaultEntry,
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use zip::write::SimpleFileOptions;

pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn inspect_files_cmd(
    paths: Vec<String>,
    vault_passwords: Vec<VaultEntry>,
) -> Vec<InspectedFile> {
    inspector::inspect_files_batch(&paths, &vault_passwords)
}

pub fn convert_single_file_cmd(
    input_path: String,
    target_format: String,
    options: ConversionOptions,
) -> Result<SingleConversionResponse, String> {
    converters::convert_single_dispatch(&input_path, &target_format, &options)
}

pub fn merge_images_to_pdf_cmd(
    input_paths: Vec<String>,
    output_dir: String,
    output_file_name: String,
    quality: u8,
) -> Result<PdfToolkitResult, String> {
    converters::image_conv::merge_images_into_pdf(
        &input_paths,
        &output_dir,
        &output_file_name,
        quality,
    )
}

pub fn run_pdf_toolkit_cmd(request: PdfToolkitRequest) -> Result<PdfToolkitResult, String> {
    pdf_toolkit::execute_pdf_toolkit(&request)
}

pub fn unlock_protected_file_cmd(
    file_id: String,
    input_path: String,
    output_dir: String,
    manual_password: Option<String>,
    vault_entries: Vec<VaultEntry>,
) -> Result<UnlockResult, String> {
    unlocker::unlock_file_permanent(
        &file_id,
        &input_path,
        &output_dir,
        manual_password.as_deref(),
        &vault_entries,
    )
}

pub fn load_vault_entries_cmd() -> Vec<VaultEntry> {
    unlocker::load_vault()
}

pub fn save_vault_entries_cmd(entries: Vec<VaultEntry>) -> Result<Vec<VaultEntry>, String> {
    unlocker::save_vault(&entries)
}

pub fn check_dependencies_cmd() -> Vec<DependencyStatus> {
    deps::check_all_dependencies()
}

pub fn install_dependency_cmd(dep_name: String) -> Result<DependencyStatus, String> {
    deps::install_or_provision_dependency(&dep_name)
}

pub fn read_file_base64_cmd(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Fájl olvasási hiba ({}): {}", path, e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

pub fn write_file_base64_cmd(
    output_path: String,
    base64_data: String,
) -> Result<(String, u64), String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_data.trim())
        .map_err(|e| format!("Base64 dekódolási hiba: {}", e))?;
    let p = PathBuf::from(&output_path);
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&p, &bytes).map_err(|e| format!("Fájl írási hiba: {}", e))?;
    Ok((p.to_string_lossy().to_string(), bytes.len() as u64))
}

/// Generates realistic multi-category demo files (Open PDF, Owner-Locked PDF, Vault-Protected PDF,
/// PNG image, CSV spreadsheet, and ZIP archive) so the user can test every OmniConvert engine immediately.
pub fn generate_demo_samples_cmd() -> Result<Vec<String>, String> {
    let sample_dir = PathBuf::from("./omniconvert-samples");
    std::fs::create_dir_all(&sample_dir).map_err(|e| e.to_string())?;

    let mut paths = Vec::new();

    // 1. Owner-Locked PDF (Permission locked -> Instant passwordless unlock!)
    let owner_pdf = sample_dir.join("NAV_Szamla_Masolasvedett_OwnerLock.pdf");
    create_sample_pdf(
        &owner_pdf,
        "OMNICONVERT PENZUGYI SZAMLA #2026-00482 (OWNER LOCK)",
        "Szallito: OmniConvert Kft. | Fizetendo: 600 000 Ft | Masolasvedett PDF",
        Some("OWNER-LOCK-ONLY"),
    )?;
    paths.push(owner_pdf.to_string_lossy().to_string());

    // 2. Auto-Unlock Vault Protected PDF (Matches default vault password "12345678")
    let vault_pdf = sample_dir.join("Banki_Kivonat_Jelszavas_AutoVault.pdf");
    create_sample_pdf(
        &vault_pdf,
        "HAVI BANKI SZAMLAKIVONAT 2026/11 (AUTO-UNLOCK VAULT)",
        "Nyito egyenleg: 2 450 000 Ft | Zaro egyenleg: 3 120 000 Ft | Adoszam: 12345678",
        Some("12345678"),
    )?;
    paths.push(vault_pdf.to_string_lossy().to_string());

    // 3. Open 2-Page Editable PDF
    let open_pdf = sample_dir.join("Szerzodes_Tervezet_Nyitott_2Oldal.pdf");
    create_sample_pdf(
        &open_pdf,
        "SZOFTVERFEJLESZTESI KERETSZERZODES (NYITOTT PDF)",
        "1. Fejezet: A vallalkozo 100% offline Rust & Tauri v2 alkalmazast szallit.",
        None,
    )?;
    paths.push(open_pdf.to_string_lossy().to_string());

    // 4. High-res PNG diagram image
    let png_path = sample_dir.join("Architektura_Diagram_4K.png");
    let mut img = RgbaImage::from_pixel(640, 360, Rgba([24, 24, 27, 255]));
    for y in 40..320 {
        for x in 60..580 {
            if x == 60 || x == 579 || y == 40 || y == 319 {
                img.put_pixel(x, y, Rgba([37, 99, 235, 255]));
            }
        }
    }
    img.save(&png_path).map_err(|e| e.to_string())?;
    paths.push(png_path.to_string_lossy().to_string());

    // 5. Financial CSV Table
    let csv_path = sample_dir.join("Negyedeves_Penzugyi_Kimutatas.csv");
    let csv_content = "Tetel,Kategoria,Netto_HUF,AFA_HUF,Brutto_HUF,Statusz\n\
        OmniConvert Enterprise Licenc,Szoftver,480000,129600,609600,Kifizetve\n\
        Rust PDF Engine Tamogatas,Mernoki Dij,250000,67500,317500,Kifizetve\n\
        Cross-Platform macOS & Windows CI,Infrastruktura,120000,32400,152400,Jovahagyva\n";
    std::fs::write(&csv_path, csv_content).map_err(|e| e.to_string())?;
    paths.push(csv_path.to_string_lossy().to_string());

    // 6. Project Archive (ZIP)
    let zip_path = sample_dir.join("Projekt_Dokumentacio_Csomag.zip");
    {
        let f = File::create(&zip_path).map_err(|e| e.to_string())?;
        let mut zw = zip::ZipWriter::new(f);
        let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zw.start_file("README.txt", opts).map_err(|e| e.to_string())?;
        zw.write_all(b"OmniConvert Offline Archivum Tesztcsomag\n")
            .map_err(|e| e.to_string())?;
        zw.start_file("config.json", opts).map_err(|e| e.to_string())?;
        zw.write_all(b"{\"offline\": true, \"engine\": \"rust-tauri-v2\"}\n")
            .map_err(|e| e.to_string())?;
        zw.finish().map_err(|e| e.to_string())?;
    }
    paths.push(zip_path.to_string_lossy().to_string());

    Ok(paths)
}

fn create_sample_pdf(
    out_path: &PathBuf,
    heading: &str,
    body_line: &str,
    lock_mode: Option<&str>,
) -> Result<(), String> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id }
    });

    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 14.into()]),
            Operation::new("Td", vec![45.into(), 775.into()]),
            Operation::new("Tj", vec![Object::string_literal(heading)]),
            Operation::new("ET", vec![]),
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 11.into()]),
            Operation::new("Td", vec![45.into(), 735.into()]),
            Operation::new("Tj", vec![Object::string_literal(body_line)]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(
        dictionary! {},
        content.encode().unwrap_or_default(),
    ));

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        "Contents" => content_id,
        "Resources" => resources_id,
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    if let Some(mode) = lock_mode {
        let enc_id = doc.add_object(dictionary! {
            "Filter" => "Standard",
            "V" => 2,
            "R" => 3,
            "P" => -3904,
            "U" => Object::string_literal(mode),
        });
        doc.trailer.set("Encrypt", enc_id);
    }

    doc.save(out_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EncryptionStatus;

    #[test]
    fn test_end_to_end_inspector_unlocker_and_converters() {
        let sample_paths = generate_demo_samples_cmd().expect("Sample generation should succeed");
        assert!(sample_paths.len() >= 5);

        let vault = load_vault_entries_cmd();
        let inspected = inspect_files_cmd(sample_paths.clone(), vault.clone());
        assert_eq!(inspected.len(), sample_paths.len());

        // Verify Owner-Locked PDF detection & instant passwordless unlocking
        let owner_pdf = inspected
            .iter()
            .find(|f| f.name.contains("OwnerLock"))
            .expect("Owner-locked PDF should be inspected");
        assert_eq!(owner_pdf.encryption_status, EncryptionStatus::OwnerLocked);

        let unlock_res = unlock_protected_file_cmd(
            owner_pdf.id.clone(),
            owner_pdf.path.clone(),
            "./omniconvert-output".to_string(),
            None,
            vault.clone(),
        )
        .expect("Passwordless Owner Lock stripping should succeed");
        assert!(unlock_res.success);
        assert_eq!(unlock_res.method_used, "owner_lock_strip");

        // Verify Auto-Unlock Vault PDF detection & automatic unlocking
        let vault_pdf = inspected
            .iter()
            .find(|f| f.name.contains("AutoVault"))
            .expect("Auto-Vault PDF should be inspected");
        assert_eq!(vault_pdf.encryption_status, EncryptionStatus::AutoUnlocked);

        // Verify CSV -> XLSX and CSV -> JSON conversion
        let csv_file = inspected
            .iter()
            .find(|f| f.detected_format == "csv")
            .expect("CSV file should be inspected");
        let opts = ConversionOptions {
            output_dir: "./omniconvert-output".to_string(),
            image_quality: 90,
            resize_width: None,
            resize_height: None,
            keep_aspect_ratio: true,
            strip_exif: true,
            merge_images_to_single_pdf: false,
            video_resolution: "original".to_string(),
            video_crf: 23,
            hardware_acceleration: "auto".to_string(),
            audio_bitrate_kbps: 192,
        };
        let xlsx_res =
            convert_single_file_cmd(csv_file.path.clone(), "xlsx".to_string(), opts.clone())
                .expect("CSV to XLSX conversion should succeed");
        assert!(xlsx_res.output_size_bytes > 0);
    }

    #[test]
    fn test_pdf_toolkit_merge_split_compress_rotate() {
        let sample_paths = generate_demo_samples_cmd().expect("Sample generation should succeed");
        let open_pdf = sample_paths
            .iter()
            .find(|p| p.contains("Nyitott"))
            .expect("Open PDF sample should exist")
            .clone();

        // 1. Merge two PDFs
        let merge_res = run_pdf_toolkit_cmd(PdfToolkitRequest {
            operation: "merge".to_string(),
            input_paths: vec![open_pdf.clone(), open_pdf.clone()],
            output_dir: "./omniconvert-output".to_string(),
            split_mode: None,
            page_ranges: None,
            merge_ranges_into_single_file: None,
            every_n_pages: None,
            compression_profile: None,
            rotation_degrees: None,
            target_pages: None,
            image_format: None,
        })
        .expect("PDF merge should succeed");
        assert!(merge_res.success);
        let merged_path = merge_res.output_paths[0].clone();

        // 2. Compress merged PDF
        let comp_res = run_pdf_toolkit_cmd(PdfToolkitRequest {
            operation: "compress".to_string(),
            input_paths: vec![merged_path.clone()],
            output_dir: "./omniconvert-output".to_string(),
            split_mode: None,
            page_ranges: None,
            merge_ranges_into_single_file: None,
            every_n_pages: None,
            compression_profile: Some("balanced".to_string()),
            rotation_degrees: None,
            target_pages: None,
            image_format: None,
        })
        .expect("PDF compression should succeed");
        assert!(comp_res.success);
        assert!(comp_res.output_size_bytes > 0);

        // 3. Split merged PDF in burst mode
        let split_res = run_pdf_toolkit_cmd(PdfToolkitRequest {
            operation: "split".to_string(),
            input_paths: vec![merged_path.clone()],
            output_dir: "./omniconvert-output".to_string(),
            split_mode: Some("burst".to_string()),
            page_ranges: None,
            merge_ranges_into_single_file: Some(false),
            every_n_pages: None,
            compression_profile: None,
            rotation_degrees: None,
            target_pages: None,
            image_format: None,
        })
        .expect("PDF burst split should succeed");
        assert!(split_res.success);
        assert_eq!(split_res.output_paths.len(), 2);

        // 4. Rotate PDF pages 90 degrees
        let rot_res = run_pdf_toolkit_cmd(PdfToolkitRequest {
            operation: "rotate".to_string(),
            input_paths: vec![open_pdf],
            output_dir: "./omniconvert-output".to_string(),
            split_mode: None,
            page_ranges: None,
            merge_ranges_into_single_file: None,
            every_n_pages: None,
            compression_profile: None,
            rotation_degrees: Some(90),
            target_pages: None,
            image_format: None,
        })
        .expect("PDF rotate should succeed");
        assert!(rot_res.success);
    }

    #[test]
    fn test_image_and_archive_pipelines() {
        let sample_paths = generate_demo_samples_cmd().expect("Sample generation should succeed");
        let png_path = sample_paths
            .iter()
            .find(|p| p.ends_with(".png"))
            .expect("PNG sample should exist")
            .clone();
        let zip_path = sample_paths
            .iter()
            .find(|p| p.ends_with(".zip"))
            .expect("ZIP sample should exist")
            .clone();

        let opts = ConversionOptions {
            output_dir: "./omniconvert-output".to_string(),
            image_quality: 85,
            resize_width: Some(320),
            resize_height: Some(180),
            keep_aspect_ratio: true,
            strip_exif: true,
            merge_images_to_single_pdf: false,
            video_resolution: "original".to_string(),
            video_crf: 23,
            hardware_acceleration: "auto".to_string(),
            audio_bitrate_kbps: 192,
        };

        // PNG -> WEBP with resize & EXIF stripping
        let webp_res = convert_single_file_cmd(png_path, "webp".to_string(), opts.clone())
            .expect("PNG to WEBP conversion should succeed");
        assert!(webp_res.output_size_bytes > 0);

        // ZIP -> TAR.GZ archive repackaging
        let targz_res = convert_single_file_cmd(zip_path, "tar.gz".to_string(), opts)
            .expect("ZIP to TAR.GZ repackaging should succeed");
        assert!(targz_res.output_size_bytes > 0);
    }
}

