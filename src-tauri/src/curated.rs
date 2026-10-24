use crate::model::Source;
use crate::net::read_capped;
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

const CATALOG_URL: &str = "https://raw.githubusercontent.com/0x89-y/Acy/main/curated.json";

// The real catalog is ~100 KB; this only stops a runaway download.
const MAX_CATALOG_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogUpdate {
    pub updated: bool,
    pub available: bool,
    pub version: u32,
    pub app_count: usize,
    pub message: String,
}

fn known_source(entry: &serde_json::Value) -> bool {
    entry
        .get("source")
        .is_some_and(|s| serde_json::from_value::<Source>(s.clone()).is_ok())
}

// Catalogs saved before the `local` source was removed can still contain it.
// Drop those entries (promoting a known alternate when the primary is gone)
// instead of failing the whole file and losing every custom app with it.
fn strip_unknown_sources(catalog: &mut serde_json::Value) {
    let Some(cats) = catalog.get_mut("categories").and_then(|c| c.as_array_mut()) else {
        return;
    };
    for cat in cats {
        let Some(apps) = cat.get_mut("apps").and_then(|a| a.as_array_mut()) else {
            continue;
        };
        for app in apps.iter_mut() {
            if let Some(alts) = app.get_mut("alternates").and_then(|a| a.as_array_mut()) {
                alts.retain(known_source);
            }
            if known_source(app) {
                continue;
            }
            let promoted = app
                .get_mut("alternates")
                .and_then(|a| a.as_array_mut())
                .filter(|a| !a.is_empty())
                .map(|a| a.remove(0));
            if let (Some(alt), Some(obj)) = (promoted, app.as_object_mut()) {
                obj.insert("source".into(), alt["source"].clone());
                obj.insert("id".into(), alt["id"].clone());
            }
        }
        apps.retain(known_source);
    }
}

// Write to a sibling temp file, then rename over the target, so a crash
// mid-write can't leave a truncated catalog behind.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

fn parse_catalog(bytes: &[u8]) -> serde_json::Result<CuratedFile> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
    strip_unknown_sources(&mut value);
    serde_json::from_value(value)
}

fn read_curated(path: &Path) -> Option<CuratedFile> {
    let text = std::fs::read(path).ok()?;
    match parse_catalog(&text) {
        Ok(file) => Some(file),
        Err(e) => {
            eprintln!("curated.json at {} is invalid: {e}", path.display());
            None
        }
    }
}

fn empty() -> CuratedFile {
    CuratedFile {
        version: 0,
        categories: Vec::new(),
    }
}

fn downloaded_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("curated-catalog.json"))
}

fn legacy_cache_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_cache_dir()
        .ok()
        .map(|dir| dir.join("curated-remote.json"))
}

fn read_downloaded(app: &AppHandle) -> Option<CuratedFile> {
    let path = downloaded_path(app)?;
    if let Some(file) = read_curated(&path) {
        return Some(file);
    }
    let legacy = legacy_cache_path(app)?;
    let file = read_curated(&legacy)?;
    if let Ok(bytes) = std::fs::read(&legacy) {
        let _ = std::fs::write(&path, bytes);
    }
    Some(file)
}

fn pick_base(downloaded: Option<CuratedFile>) -> CuratedFile {
    downloaded.unwrap_or_else(empty)
}

fn load_base(app: &AppHandle) -> CuratedFile {
    if let Some(custom) = read_custom_catalog(app) {
        return custom.catalog;
    }
    // Dev builds read the repo copy so catalog edits show up live. Release builds
    // must not: the path is the build machine's, and a file planted there would
    // replace the downloaded catalog.
    #[cfg(debug_assertions)]
    {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("curated.json");
        if let Some(file) = read_curated(&repo) {
            return file;
        }
    }
    pick_base(read_downloaded(app))
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
    write_atomic(&path, serde_json::to_string_pretty(&tagged)?.as_bytes())?;
    Ok(path)
}

fn app_count(file: &CuratedFile) -> usize {
    file.categories.iter().map(|c| c.apps.len()).sum()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStatus {
    pub present: bool,
    pub custom: bool,
    pub version: u32,
    pub app_count: usize,
}

pub fn status(app: &AppHandle) -> CatalogStatus {
    let base = load_base(app);
    CatalogStatus {
        present: base.version > 0 || !base.categories.is_empty(),
        custom: read_custom_catalog(app).is_some(),
        version: base.version,
        app_count: app_count(&base),
    }
}

pub async fn update_remote(app: &AppHandle, apply: bool) -> Result<CatalogUpdate, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("Acy/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(CATALOG_URL)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Couldn't download the catalog: {e}"))?;
    let json_bytes = read_capped(resp, MAX_CATALOG_BYTES)
        .await
        .ok_or("Couldn't read the catalog (download failed or too large).")?;

    let remote: CuratedFile = parse_catalog(&json_bytes)
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

    let path = downloaded_path(app).ok_or("Couldn't find a place to store the catalog.")?;
    write_atomic(&path, &json_bytes).map_err(|e| format!("Couldn't save the catalog: {e}"))?;

    Ok(CatalogUpdate {
        updated: true,
        available: false,
        version: remote.version,
        app_count: remote_count,
        message: format!("Updated to catalog v{} ({remote_count} apps).", remote.version),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomCatalog {
    pub source: String,
    pub is_url: bool,
    pub catalog: CuratedFile,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomCatalogInfo {
    pub source: String,
    pub is_url: bool,
    pub version: u32,
    pub app_count: usize,
}

fn custom_catalog_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("custom-catalog.json"))
}

fn read_custom_catalog(app: &AppHandle) -> Option<CustomCatalog> {
    let text = std::fs::read(custom_catalog_path(app)?).ok()?;
    let mut value: serde_json::Value = serde_json::from_slice(&text).ok()?;
    if let Some(catalog) = value.get_mut("catalog") {
        strip_unknown_sources(catalog);
    }
    serde_json::from_value(value).ok()
}

fn info_of(c: &CustomCatalog) -> CustomCatalogInfo {
    CustomCatalogInfo {
        source: c.source.clone(),
        is_url: c.is_url,
        version: c.catalog.version,
        app_count: app_count(&c.catalog),
    }
}

pub fn custom_info(app: &AppHandle) -> Option<CustomCatalogInfo> {
    read_custom_catalog(app).as_ref().map(info_of)
}

pub async fn set_custom(
    app: &AppHandle,
    source: String,
    is_url: bool,
) -> Result<CustomCatalogInfo, String> {
    let bytes = if is_url {
        let client = reqwest::Client::builder()
            .user_agent(concat!("Acy/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .get(source.trim())
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Couldn't download the catalog: {e}"))?;
        read_capped(resp, MAX_CATALOG_BYTES)
            .await
            .ok_or("Couldn't read the catalog (download failed or too large).")?
    } else {
        std::fs::read(source.trim()).map_err(|e| format!("Couldn't read the file: {e}"))?
    };

    let catalog: CuratedFile = parse_catalog(&bytes)
        .map_err(|e| format!("That's not a valid catalog (same format as the built-in one): {e}"))?;

    let custom = CustomCatalog {
        source: source.trim().to_string(),
        is_url,
        catalog,
    };
    let path = custom_catalog_path(app).ok_or("Couldn't find a place to store the catalog.")?;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text = serde_json::to_string(&custom).map_err(|e| e.to_string())?;
    write_atomic(&path, text.as_bytes()).map_err(|e| format!("Couldn't save the catalog: {e}"))?;
    Ok(info_of(&custom))
}

pub fn clear_custom(app: &AppHandle) -> Result<(), String> {
    if let Some(path) = custom_catalog_path(app) {
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
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
    fn pick_base_uses_the_download_or_nothing() {
        let none = pick_base(None);
        assert_eq!(none.version, 0);
        assert!(none.categories.is_empty());

        let file = parse(r#"{"version":5,"categories":[{"id":"x","title":"X","apps":[]}]}"#);
        assert_eq!(pick_base(Some(file)).version, 5);
    }

    #[test]
    fn merge_onto_empty_base_keeps_only_customs() {
        let saved = parse(
            r#"{"version":1,"categories":[{"id":"browsers","title":"Browsers","apps":[
                {"id":"Mozilla.Firefox","source":"winget","custom":false},
                {"id":"Brave.Brave","source":"winget","custom":true}
            ]}]}"#,
        );
        let merged = merge(empty(), &saved);
        assert_eq!(ids(&merged, "browsers"), vec!["Brave.Brave"]);
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
    fn parse_catalog_drops_removed_local_source() {
        let file = parse_catalog(
            br#"{"version":1,"categories":[{"id":"mine","title":"Mine","apps":[
                {"id":"C:\\setup.exe","source":"local","custom":true},
                {"id":"","source":"local","custom":true,
                 "alternates":[{"source":"local","id":""},{"source":"scoop","id":"foo"}]},
                {"id":"Git.Git","source":"winget","custom":true,
                 "alternates":[{"source":"local","id":"x.msi"},{"source":"choco","id":"git"}]}
            ]}]}"#,
        )
        .unwrap();
        let apps = &file.categories[0].apps;
        assert_eq!(apps.len(), 2);
        assert_eq!((apps[0].source, apps[0].id.as_str()), (Source::Scoop, "foo"));
        assert!(apps[0].alternates.is_empty());
        assert_eq!(apps[1].id, "Git.Git");
        assert_eq!(
            apps[1].alternates,
            vec![CuratedVariant { source: Source::Choco, id: "git".into() }]
        );
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
