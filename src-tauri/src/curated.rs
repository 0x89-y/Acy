use crate::model::Source;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// An additional package manager the same curated app can be installed from.
/// Ids are manager-specific, so each variant carries its own id.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CuratedVariant {
    pub source: Source,
    pub id: String,
}

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
    /// Other managers the same app is available from. The primary `source`/`id`
    /// plus these form the full set of install options shown on the card.
    #[serde(default)]
    pub alternates: Vec<CuratedVariant>,
    /// Free-form labels (e.g. "open source", "free", "chromium", a license) used
    /// for display and filtering on Discover.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional donation / "support the developer" URL.
    #[serde(default)]
    pub donate: Option<String>,
    /// True when the user added this app (it is not in the bundled catalog).
    /// Set by `save`; lets a new app version refresh the built-in apps while
    /// keeping the user's own additions.
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

/// Embedded fallback, guaranteed to parse. Used if no external file is found.
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

/// The hardcoded catalog that ships with this build and is the source of truth
/// for built-in apps: the repo-root file during development (live edits, no
/// rebuild), the bundled resource copy in a shipped app, otherwise the embedded
/// copy. Deliberately does NOT include the per-user AppData file.
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

/// Path to the per-user saved catalog (the in-app editor writes here).
fn appdata_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("curated.json"))
}

/// Identity of an app for matching across catalogs: source + lowercased id.
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

/// Overlay the user's custom apps from `saved` onto the current hardcoded
/// `base`. Built-in apps always come from `base`, so new built-ins appear and
/// removed ones disappear on update; only apps the user added (tagged `custom`)
/// are carried over, and never duplicate a built-in.
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

/// Load the curated catalog: the bundled base with the user's custom apps merged
/// in. `ACY_CURATED` still acts as a full override for testing.
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

/// Write the catalog to the per-user config dir, tagging each app as custom
/// (user-added) or built-in relative to the current bundled catalog so `load`
/// can keep customs across updates. Returns the path written.
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
        // New build adds Chrome as a built-in browser.
        let base = parse(
            r#"{"version":2,"categories":[
                {"id":"browsers","title":"Browsers","apps":[
                    {"id":"Mozilla.Firefox","source":"winget"},
                    {"id":"Google.Chrome","source":"winget"}
                ]}
            ]}"#,
        );
        // User previously saved: Firefox (built-in), Brave (custom), and a whole
        // custom category.
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
        assert!(browsers.contains(&"Google.Chrome".to_string())); // new built-in shows
        assert!(browsers.contains(&"Brave.Brave".to_string())); // custom kept
        assert_eq!(
            browsers.iter().filter(|i| *i == "Mozilla.Firefox").count(),
            1 // no duplicate
        );
        assert_eq!(ids(&merged, "mine"), vec!["Some.App"]); // custom category kept
    }

    #[test]
    fn merge_does_not_resurrect_removed_builtin() {
        // Firefox was dropped from the bundled catalog in this version.
        let base = parse(r#"{"version":2,"categories":[{"id":"browsers","title":"Browsers","apps":[]}]}"#);
        // The old saved copy still lists it, tagged as a built-in.
        let saved = parse(
            r#"{"version":1,"categories":[{"id":"browsers","title":"Browsers","apps":[
                {"id":"Mozilla.Firefox","source":"winget","custom":false}
            ]}]}"#,
        );
        let merged = merge(base, &saved);
        assert!(ids(&merged, "browsers").is_empty());
    }
}
