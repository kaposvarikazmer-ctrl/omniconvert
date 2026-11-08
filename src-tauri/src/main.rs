#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use omniconvert_lib::models::{
    ConversionOptions, DependencyStatus, InspectedFile, PdfToolkitRequest, PdfToolkitResult,
    SingleConversionResponse, UnlockResult, VaultEntry,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InspectPayload {
    paths: Vec<String>,
    #[serde(default)]
    vault_passwords: Vec<VaultEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConvertSinglePayload {
    #[allow(dead_code)]
    file_id: Option<String>,
    input_path: String,
    target_format: String,
    options: ConversionOptions,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MergeImagesPayload {
    input_paths: Vec<String>,
    output_dir: String,
    output_file_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PdfToolkitPayload {
    request: PdfToolkitRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnlockPayload {
    file_id: String,
    input_path: String,
    output_dir: String,
    manual_password: Option<String>,
    #[serde(default)]
    vault_entries: Vec<VaultEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveVaultPayload {
    entries: Vec<VaultEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallDepPayload {
    dep_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadBase64Payload {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteBase64Payload {
    output_path: String,
    base64_data: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WriteBase64Response {
    output_path: String,
    size_bytes: u64,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod desktop_tauri {
    use super::*;

    #[tauri::command]
    pub fn inspect_files(
        paths: Vec<String>,
        vault_passwords: Vec<VaultEntry>,
    ) -> Vec<InspectedFile> {
        omniconvert_lib::inspect_files_cmd(paths, vault_passwords)
    }

    #[tauri::command]
    pub fn convert_single_file(
        _file_id: String,
        input_path: String,
        target_format: String,
        options: ConversionOptions,
    ) -> Result<SingleConversionResponse, String> {
        omniconvert_lib::convert_single_file_cmd(input_path, target_format, options)
    }

    #[tauri::command]
    pub fn merge_images_to_pdf(
        input_paths: Vec<String>,
        output_dir: String,
        output_file_name: String,
    ) -> Result<PdfToolkitResult, String> {
        omniconvert_lib::merge_images_to_pdf_cmd(input_paths, output_dir, output_file_name, 90)
    }

    #[tauri::command]
    pub fn run_pdf_toolkit(request: PdfToolkitRequest) -> Result<PdfToolkitResult, String> {
        omniconvert_lib::run_pdf_toolkit_cmd(request)
    }

    #[tauri::command]
    pub fn unlock_protected_file(
        file_id: String,
        input_path: String,
        output_dir: String,
        manual_password: Option<String>,
        vault_entries: Vec<VaultEntry>,
    ) -> Result<UnlockResult, String> {
        omniconvert_lib::unlock_protected_file_cmd(
            file_id,
            input_path,
            output_dir,
            manual_password,
            vault_entries,
        )
    }

    #[tauri::command]
    pub fn load_vault_entries() -> Vec<VaultEntry> {
        omniconvert_lib::load_vault_entries_cmd()
    }

    #[tauri::command]
    pub fn save_vault_entries(entries: Vec<VaultEntry>) -> Result<Vec<VaultEntry>, String> {
        omniconvert_lib::save_vault_entries_cmd(entries)
    }

    #[tauri::command]
    pub fn check_dependencies() -> Vec<DependencyStatus> {
        omniconvert_lib::check_dependencies_cmd()
    }

    #[tauri::command]
    pub fn install_dependency(dep_name: String) -> Result<DependencyStatus, String> {
        omniconvert_lib::install_dependency_cmd(dep_name)
    }

    #[tauri::command]
    pub fn read_file_base64(path: String) -> Result<String, String> {
        omniconvert_lib::read_file_base64_cmd(path)
    }

    #[tauri::command]
    pub fn write_file_base64(
        output_path: String,
        base64_data: String,
    ) -> Result<WriteBase64Response, String> {
        let (out, sz) = omniconvert_lib::write_file_base64_cmd(output_path, base64_data)?;
        Ok(WriteBase64Response {
            output_path: out,
            size_bytes: sz,
        })
    }

    #[tauri::command]
    pub fn generate_demo_samples() -> Result<Vec<String>, String> {
        omniconvert_lib::generate_demo_samples_cmd()
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            desktop_tauri::inspect_files,
            desktop_tauri::convert_single_file,
            desktop_tauri::merge_images_to_pdf,
            desktop_tauri::run_pdf_toolkit,
            desktop_tauri::unlock_protected_file,
            desktop_tauri::load_vault_entries,
            desktop_tauri::save_vault_entries,
            desktop_tauri::check_dependencies,
            desktop_tauri::install_dependency,
            desktop_tauri::read_file_base64,
            desktop_tauri::write_file_base64,
            desktop_tauri::generate_demo_samples,
        ])
        .run(tauri::generate_context!())
        .expect("Hiba történt az OmniConvert Tauri v2 alkalmazás futtatásakor");
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[tokio::main]
async fn main() {
    use axum::{
        extract::Path as AxumPath,
        http::StatusCode,
        response::{IntoResponse, Response},
        routing::post,
        Json, Router,
    };
    use tower_http::cors::CorsLayer;

    async fn ipc_handler(
        AxumPath(cmd): AxumPath<String>,
        body: String,
    ) -> Response {
        let res: Result<serde_json::Value, String> = tokio::task::spawn_blocking(move || {
            match cmd.as_str() {
                "inspect_files" => {
                    let p: InspectPayload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let out = omniconvert_lib::inspect_files_cmd(p.paths, p.vault_passwords);
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "convert_single_file" => {
                    let p: ConvertSinglePayload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let out = omniconvert_lib::convert_single_file_cmd(
                        p.input_path,
                        p.target_format,
                        p.options,
                    )?;
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "merge_images_to_pdf" => {
                    let p: MergeImagesPayload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let out = omniconvert_lib::merge_images_to_pdf_cmd(
                        p.input_paths,
                        p.output_dir,
                        p.output_file_name,
                        90,
                    )?;
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "run_pdf_toolkit" => {
                    let p: PdfToolkitPayload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let out = omniconvert_lib::run_pdf_toolkit_cmd(p.request)?;
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "unlock_protected_file" => {
                    let p: UnlockPayload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let out = omniconvert_lib::unlock_protected_file_cmd(
                        p.file_id,
                        p.input_path,
                        p.output_dir,
                        p.manual_password,
                        p.vault_entries,
                    )?;
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "load_vault_entries" => {
                    let out: Vec<VaultEntry> = omniconvert_lib::load_vault_entries_cmd();
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "save_vault_entries" => {
                    let p: SaveVaultPayload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let out = omniconvert_lib::save_vault_entries_cmd(p.entries)?;
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "check_dependencies" => {
                    let out: Vec<DependencyStatus> = omniconvert_lib::check_dependencies_cmd();
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "install_dependency" => {
                    let p: InstallDepPayload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let out = omniconvert_lib::install_dependency_cmd(p.dep_name)?;
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "read_file_base64" => {
                    let p: ReadBase64Payload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let out = omniconvert_lib::read_file_base64_cmd(p.path)?;
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                "write_file_base64" => {
                    let p: WriteBase64Payload =
                        serde_json::from_str(&body).map_err(|e| e.to_string())?;
                    let (out_path, size_bytes) =
                        omniconvert_lib::write_file_base64_cmd(p.output_path, p.base64_data)?;
                    serde_json::to_value(WriteBase64Response {
                        output_path: out_path,
                        size_bytes,
                    })
                    .map_err(|e| e.to_string())
                }
                "generate_demo_samples" => {
                    let out = omniconvert_lib::generate_demo_samples_cmd()?;
                    serde_json::to_value(out).map_err(|e| e.to_string())
                }
                other => Err(format!("Ismeretlen IPC parancs: {}", other)),
            }
        })
        .await
        .unwrap_or_else(|e| Err(format!("Task panic: {}", e)));

        match res {
            Ok(val) => (StatusCode::OK, Json(val)).into_response(),
            Err(err_msg) => (StatusCode::BAD_REQUEST, err_msg).into_response(),
        }
    }

    let app = Router::new()
        .route("/api/ipc/:cmd", post(ipc_handler))
        .layer(CorsLayer::permissive());

    let addr = "127.0.0.1:1422";
    println!("OmniConvert Rust IPC Backend listening on http://{}", addr);
    if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
        let _ = axum::serve(listener, app).await;
    }
}
