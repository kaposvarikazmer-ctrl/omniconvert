pub mod models;

use models::DependencyStatus;

pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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
