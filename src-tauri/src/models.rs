use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionStatus {
    Open,
    OwnerLocked,
    UserLocked,
    AutoUnlocked,
    ManuallyUnlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfMetadata {
    pub page_count: usize,
    pub has_text_layer: bool,
    pub pdf_version: String,
    pub encryption_status: EncryptionStatus,
    pub permissions_summary: Vec<String>,
    pub unlocked_with_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMetadata {
    pub item_count: usize,
    pub uncompressed_size: u64,
    pub is_encrypted: bool,
    pub encryption_status: EncryptionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_secs: Option<f64>,
    pub codec: Option<String>,
    pub bitrate_kbps: Option<u32>,
    pub has_exif: Option<bool>,
    pub color_space: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DataMetadata {
    pub row_count: Option<usize>,
    pub column_count: Option<usize>,
    pub columns: Option<Vec<String>>,
    pub sheet_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectedFile {
    pub id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub detected_mime: String,
    pub detected_format: String,
    pub magic_bytes_hex: String,
    pub extension_mismatch: bool,
    pub category: String,
    pub size_bytes: u64,
    pub encryption_status: EncryptionStatus,
    pub status_badge_text: String,
    pub available_targets: Vec<String>,
    pub selected_target: String,
    pub pdf_meta: Option<PdfMetadata>,
    pub archive_meta: Option<ArchiveMetadata>,
    pub media_meta: Option<MediaMetadata>,
    pub data_meta: Option<DataMetadata>,
    pub conversion_status: String,
    pub progress_percent: u8,
    pub output_path: Option<String>,
    pub output_size_bytes: Option<u64>,
    pub savings_percent: Option<f64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionOptions {
    pub output_dir: String,
    pub image_quality: u8,
    pub resize_width: Option<u32>,
    pub resize_height: Option<u32>,
    pub keep_aspect_ratio: bool,
    pub strip_exif: bool,
    pub merge_images_to_single_pdf: bool,
    pub video_resolution: String,
    pub video_crf: u8,
    pub hardware_acceleration: String,
    pub audio_bitrate_kbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingleConversionResponse {
    pub output_path: String,
    pub output_size_bytes: u64,
    pub savings_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfToolkitRequest {
    pub operation: String,
    pub input_paths: Vec<String>,
    pub output_dir: String,
    pub split_mode: Option<String>,
    pub page_ranges: Option<String>,
    pub merge_ranges_into_single_file: Option<bool>,
    pub every_n_pages: Option<usize>,
    pub compression_profile: Option<String>,
    pub rotation_degrees: Option<i32>,
    pub target_pages: Option<String>,
    pub image_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfToolkitResult {
    pub success: bool,
    pub output_paths: Vec<String>,
    pub original_size_bytes: u64,
    pub output_size_bytes: u64,
    pub savings_percent: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEntry {
    pub id: String,
    pub label: String,
    pub password: String,
    pub category_hint: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockResult {
    pub file_id: String,
    pub input_path: String,
    pub success: bool,
    pub output_path: Option<String>,
    pub new_status: EncryptionStatus,
    pub method_used: String,
    pub matched_label: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub binary_path: Option<String>,
    pub bundled_or_system: String,
    pub capabilities: Vec<String>,
}
