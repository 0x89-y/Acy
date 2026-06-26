use crate::model::Source;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CuratedVariant {
    pub source: Source,
    pub id: String,
}

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
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub alternates: Vec<CuratedVariant>,
    #[serde(default)]
    pub custom: bool,
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

const DEFAULT: &str = include_str!("../../curated.json");

fn read_curated(path: &Path) -> Option<CuratedFile> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<CuratedFile>(&text) {
        Ok(file) => Some(file),
        Err(e) => {
            eprintln!("curated.json at {} is invalid: {e}", path.display());
            None
        }
    }
}

fn embedded() -> CuratedFile {
    serde_json::from_str(DEFAULT).expect("embedded curated.json must be valid")
}

fn load_base(app: &AppHandle) -> CuratedFile {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("curated.json");
    if let Some(file) = read_curated(&repo) {
        return file;
    }
    if let Ok(resource) = app
        .path()
        .resolve("curated.json", tauri::path::BaseDirectory::Resource)
    {
        if let Some(file) = read_curated(&resource) {
            return file;
        }
    }
    embedded()
}

fn appdata_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("curated.json"))
}

fn app_key(app: &CuratedApp) -> (Source, String) {
    (app.source, app.id.trim().to_lowercase())
}

fn base_keys(base: &CuratedFile) -> HashSet<(Source, String)> {
    base.categories
        .iter()
        .flat_map(|c| c.apps.iter())
        .map(app_key)
        .collect()
}

fn merge(base: CuratedFile, saved: &CuratedFile) -> CuratedFile {
    let keys = base_keys(&base);
    let mut present = keys.clone();
    let mut result = base;

    for sc in &saved.categories {
        let customs: Vec<CuratedApp> = sc
            .apps
            .iter()
            .filter(|a| a.custom && !keys.contains(&app_key(a)))
            .cloned()
            .collect();
        if customs.is_empty() {
            continue;
        }

        let idx = result.categories.iter().position(|c| c.id == sc.id);
        let target = match idx {
            Some(i) => &mut result.categories[i],
            None => {
                result.categories.push(CuratedCategory {
                    id: sc.id.clone(),
                    title: sc.title.clone(),
                    apps: Vec::new(),
                });
                result.categories.last_mut().unwrap()
            }
        };
        for mut app in customs {
            if present.insert(app_key(&app)) {
                app.custom = true;
                target.apps.push(app);
            }
        }
    }
    result
}

pub fn load(app: &AppHandle) -> CuratedFile {
    if let Ok(path) = std::env::var("ACY_CURATED") {
        if let Some(file) = read_curated(&PathBuf::from(path)) {
            return file;
        }
    }
    let base = load_base(app);
    match appdata_file(app).and_then(|p| read_curated(&p)) {
        Some(saved) => merge(base, &saved),
        None => base,
    }
}

pub fn save(app: &AppHandle, file: &CuratedFile) -> anyhow::Result<PathBuf> {
    let keys = base_keys(&load_base(app));
    let mut tagged = file.clone();
    for cat in &mut tagged.categories {
        for app in &mut cat.apps {
            app.custom = !keys.contains(&app_key(app));
        }
    }

    let dir = app.path().app_config_dir()?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("curated.json");
    std::fs::write(&path, serde_json::to_string_pretty(&tagged)?)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> CuratedFile {
        serde_json::from_str(s).unwrap()
    }

    fn ids(file: &CuratedFile, cat: &str) -> Vec<String> {
        file.categories
            .iter()
            .find(|c| c.id == cat)
            .map(|c| c.apps.iter().map(|a| a.id.clone()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn merge_refreshes_builtins_and_keeps_customs() {
        let base = parse(
            r#"{"version":2,"categories":[
                {"id":"browsers","title":"Browsers","apps":[
                    {"id":"Mozilla.Firefox","source":"winget"},
                    {"id":"Google.Chrome","source":"winget"}
                ]}
            ]}"#,
        );
        let saved = parse(
            r#"{"version":1,"categories":[
                {"id":"browsers","title":"Browsers","apps":[
                    {"id":"Mozilla.Firefox","source":"winget","custom":false},
                    {"id":"Brave.Brave","source":"winget","custom":true}
                ]},
                {"id":"mine","title":"Mine","apps":[
                    {"id":"Some.App","source":"winget","custom":true}
                ]}
            ]}"#,
        );

        let merged = merge(base, &saved);
        let browsers = ids(&merged, "browsers");
        assert!(browsers.contains(&"Google.Chrome".to_string()));
        assert!(browsers.contains(&"Brave.Brave".to_string()));
        assert_eq!(
            browsers.iter().filter(|i| *i == "Mozilla.Firefox").count(),
            1
        );
        assert_eq!(ids(&merged, "mine"), vec!["Some.App"]);
    }

    #[test]
    fn merge_does_not_resurrect_removed_builtin() {
        let base = parse(r#"{"version":2,"categories":[{"id":"browsers","title":"Browsers","apps":[]}]}"#);
        let saved = parse(
            r#"{"version":1,"categories":[{"id":"browsers","title":"Browsers","apps":[
                {"id":"Mozilla.Firefox","source":"winget","custom":false}
            ]}]}"#,
        );
        let merged = merge(base, &saved);
        assert!(ids(&merged, "browsers").is_empty());
    }
}
