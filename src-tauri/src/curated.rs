use crate::model::Source;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// One curated app. `id`/`source` are required; the rest override what the
/// manager reports, for cases where the user wants a nicer name or blurb.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CuratedApp {
    pub id: String,
    pub source: Source,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    /// Optional override for where the icon is fetched from, when the homepage's
    /// favicon is not the real app logo. A page URL or a direct image URL.
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratedCategory {
    pub id: String,
    pub title: String,
    pub apps: Vec<CuratedApp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratedFile {
    pub version: u32,
    pub categories: Vec<CuratedCategory>,
}

/// Embedded fallback, guaranteed to parse. Used if no external file is found.
const DEFAULT: &str = include_str!("../../curated.json");

/// Where to look for an external curated.json, in priority order:
/// 1. `ACY_CURATED` env var (explicit override)
/// 2. the app config dir (user edits without rebuilding a shipped app)
/// 3. the repo-root file during development (live edits, no rebuild)
/// 4. the bundled resource copy in a shipped build
fn candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(p) = std::env::var("ACY_CURATED") {
        paths.push(PathBuf::from(p));
    }
    if let Ok(dir) = app.path().app_config_dir() {
        paths.push(dir.join("curated.json"));
    }
    paths.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("curated.json"));
    if let Ok(p) = app
        .path()
        .resolve("curated.json", tauri::path::BaseDirectory::Resource)
    {
        paths.push(p);
    }
    paths
}

/// Load the curated list, falling back to the embedded default.
pub fn load(app: &AppHandle) -> CuratedFile {
    for path in candidates(app) {
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<CuratedFile>(&text) {
                Ok(parsed) => return parsed,
                Err(e) => eprintln!("curated.json at {} is invalid: {e}", path.display()),
            },
            Err(_) => continue,
        }
    }
    serde_json::from_str(DEFAULT).expect("embedded curated.json must be valid")
}
