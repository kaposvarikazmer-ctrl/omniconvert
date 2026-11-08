pub mod archive_conv;
pub mod data_conv;
pub mod image_conv;
pub mod media_doc_conv;

use crate::inspector::categorize_format;
use crate::models::{ConversionOptions, SingleConversionResponse};
use rayon::prelude::*;
use std::path::Path;

/// Unified ConversionPipeline trait implemented by all OmniConvert conversion engines.
pub trait ConversionPipeline: Send + Sync {
    fn convert(
        &self,
        input_path: &Path,
        source_format: &str,
        target_format: &str,
        options: &ConversionOptions,
    ) -> Result<SingleConversionResponse, String>;
}

pub struct UniversalConverter;

impl ConversionPipeline for UniversalConverter {
    fn convert(
        &self,
        input_path: &Path,
        source_format: &str,
        target_format: &str,
        options: &ConversionOptions,
    ) -> Result<SingleConversionResponse, String> {
        let category = categorize_format(source_format);
        match category.as_str() {
            "image" => image_conv::convert_image_file(input_path, target_format, options),
            "data" => {
                data_conv::convert_data_file(input_path, source_format, target_format, options)
            }
            "archive" => archive_conv::convert_archive_file(
                input_path,
                source_format,
                target_format,
                options,
            ),
            "video" | "audio" => {
                media_doc_conv::convert_media_file(input_path, source_format, target_format, options)
            }
            _ => media_doc_conv::convert_document_file(
                input_path,
                source_format,
                target_format,
                options,
            ),
        }
    }
}

pub fn convert_single_dispatch(
    input_path_str: &str,
    target_format: &str,
    options: &ConversionOptions,
) -> Result<SingleConversionResponse, String> {
    let input_path = Path::new(input_path_str);
    if !input_path.exists() {
        return Err(format!("A bemeneti fájl nem található: {}", input_path_str));
    }
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt")
        .to_lowercase();
    let converter = UniversalConverter;
    converter.convert(input_path, &ext, target_format, options)
}

/// Parallel batch conversion across multiple CPU threads using Rayon.
pub fn convert_batch_parallel(
    items: &[(String, String)],
    options: &ConversionOptions,
) -> Vec<Result<SingleConversionResponse, String>> {
    items
        .par_iter()
        .map(|(input_path, target_fmt)| convert_single_dispatch(input_path, target_fmt, options))
        .collect()
}
