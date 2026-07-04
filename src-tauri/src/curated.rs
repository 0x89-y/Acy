use crate::model::Source;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub tags: Vec<String>,
    #[serde(default)]
    pub donate: Option<String>,
    #[serde(default)]
    pub release_notes: Option<String>,
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

const CATALOG_URL: &str = "https://0x89-y.xyz/acy/curated.json";
const CATALOG_SIG_URL: &str = "https://0x89-y.xyz/acy/curated.json.sig";

const CATALOG_PUBKEY: &str = "RWS3VEM6J8kwYA+2cJgsbaR1llGwi+f10sRCXZU4w9SrP47w+9je9SBo";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogUpdate {
    pub updated: bool,
    pub available: bool,
    pub version: u32,
    pub app_count: usize,
    pub message: String,
}

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

fn bundled_catalog(app: &AppHandle) -> CuratedFile {
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

fn remote_cache_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_cache_dir().ok()?;
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("curated-remote.json"))
}

fn read_remote_cache(app: &AppHandle) -> Option<CuratedFile> {
    read_curated(&remote_cache_path(app)?)
}

fn pick_base(bundled: CuratedFile, remote: Option<CuratedFile>) -> CuratedFile {
    match remote {
        Some(r) if r.version >= bundled.version => r,
        _ => bundled,
    }
}

fn load_base(app: &AppHandle) -> CuratedFile {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("curated.json");
    if let Some(file) = read_curated(&repo) {
        return file;
    }
    pick_base(bundled_catalog(app), read_remote_cache(app))
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

fn same_content(a: &CuratedApp, b: &CuratedApp) -> bool {
    a.source == b.source
        && a.id == b.id
        && a.name == b.name
        && a.description == b.description
        && a.homepage == b.homepage
        && a.icon == b.icon
        && a.donate == b.donate
        && a.release_notes == b.release_notes
        && a.tags == b.tags
        && a.alternates == b.alternates
}

fn merge(base: CuratedFile, saved: &CuratedFile) -> CuratedFile {
    let keys = base_keys(&base);
    let mut present = keys.clone();
    let mut result = base;

    for sc in &saved.categories {
        for app in sc.apps.iter().filter(|a| a.custom) {
            let k = app_key(app);
            if keys.contains(&k) {
                if let Some(slot) = result
                    .categories
                    .iter_mut()
                    .flat_map(|c| c.apps.iter_mut())
                    .find(|x| app_key(x) == k)
                {
                    *slot = app.clone();
                }
                continue;
            }
            if !present.insert(k) {
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
            target.apps.push(app.clone());
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
    let base = load_base(app);
    let base_index: HashMap<(Source, String), &CuratedApp> = base
        .categories
        .iter()
        .flat_map(|c| c.apps.iter())
        .map(|a| (app_key(a), a))
        .collect();

    let mut tagged = file.clone();
    for cat in &mut tagged.categories {
        for a in &mut cat.apps {
            a.custom = match base_index.get(&app_key(a)) {
                Some(b) => !same_content(a, b),
                None => true,
            };
        }
    }

    let dir = app.path().app_config_dir()?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("curated.json");
    std::fs::write(&path, serde_json::to_string_pretty(&tagged)?)?;
    Ok(path)
}

fn app_count(file: &CuratedFile) -> usize {
    file.categories.iter().map(|c| c.apps.len()).sum()
}

pub async fn update_remote(app: &AppHandle, apply: bool) -> Result<CatalogUpdate, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("Acy/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let json_bytes = client
        .get(CATALOG_URL)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Couldn't download the catalog: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Couldn't read the catalog: {e}"))?;

    let sig_text = client
        .get(CATALOG_SIG_URL)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Couldn't download the catalog signature: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Couldn't read the catalog signature: {e}"))?;

    let sig_text = STANDARD
        .decode(sig_text.trim())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or(sig_text);
    let pk = minisign_verify::PublicKey::from_base64(CATALOG_PUBKEY)
        .map_err(|e| format!("Bad catalog public key: {e}"))?;
    let sig = minisign_verify::Signature::decode(&sig_text)
        .map_err(|_| "The catalog signature is malformed.".to_string())?;
    pk.verify(&json_bytes, &sig, false)
        .map_err(|_| "The catalog failed signature verification and was not applied.".to_string())?;

    let remote: CuratedFile = serde_json::from_slice(&json_bytes)
        .map_err(|e| format!("The downloaded catalog is not valid: {e}"))?;
    let remote_count = app_count(&remote);

    let current = load_base(app);
    if remote.version <= current.version {
        return Ok(CatalogUpdate {
            updated: false,
            available: false,
            version: current.version,
            app_count: app_count(&current),
            message: format!("You're on the latest catalog (v{}).", current.version),
        });
    }

    if !apply {
        return Ok(CatalogUpdate {
            updated: false,
            available: true,
            version: remote.version,
            app_count: remote_count,
            message: format!("Catalog v{} is available.", remote.version),
        });
    }

    let path = remote_cache_path(app).ok_or("Couldn't find a place to store the catalog.")?;
    std::fs::write(&path, &json_bytes).map_err(|e| format!("Couldn't save the catalog: {e}"))?;

    Ok(CatalogUpdate {
        updated: true,
        available: false,
        version: remote.version,
        app_count: remote_count,
        message: format!("Updated to catalog v{} ({remote_count} apps).", remote.version),
    })
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
    fn pick_base_prefers_higher_version_remote() {
        let bundled = parse(r#"{"version":3,"categories":[]}"#);
        let remote = parse(r#"{"version":5,"categories":[{"id":"x","title":"X","apps":[]}]}"#);
        assert_eq!(pick_base(bundled, Some(remote)).version, 5);

        let bundled = parse(r#"{"version":4,"categories":[]}"#);
        let remote = parse(r#"{"version":4,"categories":[]}"#);
        assert_eq!(pick_base(bundled, Some(remote)).version, 4);

        let bundled = parse(r#"{"version":6,"categories":[]}"#);
        let remote = parse(r#"{"version":2,"categories":[]}"#);
        assert_eq!(pick_base(bundled, Some(remote)).version, 6);

        let bundled = parse(r#"{"version":6,"categories":[]}"#);
        assert_eq!(pick_base(bundled, None).version, 6);
    }

    #[test]
    fn merge_overrides_builtin_in_place() {
        let base = parse(
            r#"{"version":2,"categories":[{"id":"browsers","title":"Browsers","apps":[
                {"id":"Mozilla.Firefox","source":"winget"},
                {"id":"Google.Chrome","source":"winget"}
            ]}]}"#,
        );
        let saved = parse(
            r#"{"version":1,"categories":[{"id":"browsers","title":"Browsers","apps":[
                {"id":"Mozilla.Firefox","source":"winget","name":"Firefox (mine)","custom":true}
            ]}]}"#,
        );
        let merged = merge(base, &saved);
        let browsers = &merged.categories[0].apps;
        assert_eq!(browsers.len(), 2);
        assert_eq!(browsers[0].id, "Mozilla.Firefox");
        assert_eq!(browsers[0].name.as_deref(), Some("Firefox (mine)"));
        assert_eq!(browsers[1].id, "Google.Chrome");
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
