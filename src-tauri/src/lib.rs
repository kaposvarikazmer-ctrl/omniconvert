pub mod converters;
pub mod inspector;
pub mod models;
pub mod pdf_toolkit;
pub mod unlocker;

use models::{
    ConversionOptions, DependencyStatus, InspectedFile, PdfToolkitRequest, PdfToolkitResult,
    SingleConversionResponse, UnlockResult, VaultEntry,
};

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

pub fn default_dependency_snapshot() -> Vec<DependencyStatus> {
    vec![
        DependencyStatus {
            name: "ffmpeg".to_string(),
            installed: false,
            version: None,
            binary_path: None,
            bundled_or_system: "missing".to_string(),
            capabilities: vec![
                "MP4 / MKV / MOV / WEBM / AVI".to_string(),
                "Hardveres gyorsítás (VideoToolbox / NVENC / QSV)".to_string(),
                "Hangsáv kinyerés (MP3 / FLAC / WAV / OPUS)".to_string(),
            ],
        },
        DependencyStatus {
            name: "pandoc".to_string(),
            installed: false,
            version: None,
            binary_path: None,
            bundled_or_system: "missing".to_string(),
            capabilities: vec![
                "DOCX / ODT / RTF / EPUB / FB2 / LaTeX / RST".to_string(),
                "Markdown & HTML dokumentum konverzió".to_string(),
            ],
        },
        DependencyStatus {
            name: "typst".to_string(),
            installed: false,
            version: None,
            binary_path: None,
            bundled_or_system: "missing".to_string(),
            capabilities: vec![
                "Villámgyors PDF tördelés és táblázat-renderelés".to_string(),
            ],
        },
    ]
}
