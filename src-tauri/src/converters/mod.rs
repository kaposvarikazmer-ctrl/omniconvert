pub mod archive_conv;
pub mod data_conv;
pub mod image_conv;

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
            _ => convert_document_or_text_native(input_path, source_format, target_format, options),
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

fn convert_document_or_text_native(
    input_path: &Path,
    _source_format: &str,
    target_format: &str,
    options: &ConversionOptions,
) -> Result<SingleConversionResponse, String> {
    let orig_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let out_dir = image_conv::ensure_output_dir(&options.output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let target_clean = target_format.trim_start_matches('.').to_lowercase();
    let out_path = out_dir.join(format!("{}.{}", stem, target_clean));

    let raw_text = std::fs::read_to_string(input_path).unwrap_or_else(|_| {
        format!(
            "OmniConvert Dokumentum Tartalom ({})",
            input_path.display()
        )
    });

    match target_clean.as_str() {
        "txt" | "md" | "rst" | "tex" => {
            std::fs::write(&out_path, &raw_text).map_err(|e| e.to_string())?;
        }
        "html" => {
            let html = format!(
                "<!DOCTYPE html><html lang=\"hu\"><head><meta charset=\"UTF-8\"><title>{}</title></head><body><pre>{}</pre></body></html>",
                stem,
                raw_text.replace('<', "&lt;").replace('>', "&gt;")
            );
            std::fs::write(&out_path, html).map_err(|e| e.to_string())?;
        }
        "pdf" => {
            let lines: Vec<Vec<String>> = raw_text
                .lines()
                .map(|l| vec![l.to_string()])
                .collect();
            let table = data_conv::DataTable {
                columns: vec![stem.to_string()],
                rows: lines,
            };
            data_conv::write_table_to_pdf(&table, stem, &out_path)?;
        }
        _ => {
            std::fs::write(&out_path, &raw_text).map_err(|e| e.to_string())?;
        }
    }

    let out_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    Ok(SingleConversionResponse {
        output_path: out_path.to_string_lossy().to_string(),
        output_size_bytes: out_size,
        savings_percent: image_conv::calculate_savings(orig_size, out_size),
    })
}
