use crate::model::Source;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// An additional package manager the same curated app can be installed from.
/// Ids are manager-specific, so each variant carries its own id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Optional link to the app's release notes / changelog.
    #[serde(default)]
    pub release_notes: Option<String>,
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

/// Where the hosted catalog + its signature live. Updating the catalog is a
/// matter of uploading these two files - no app rebuild needed.
const CATALOG_URL: &str = "https://0x89-y.xyz/acy/curated.json";
const CATALOG_SIG_URL: &str = "https://0x89-y.xyz/acy/curated.json.sig";

/// minisign public key that signs the catalog - the same key used for app
/// updates. It is the decoded form of the `pubkey` in tauri.conf.json (that
/// base64 decodes to a two-line pubkey file whose key line is this).
const CATALOG_PUBKEY: &str = "RWS3VEM6J8kwYA+2cJgsbaR1llGwi+f10sRCXZU4w9SrP47w+9je9SBo";

/// Result of a "check for catalog updates" run, surfaced in Settings.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogUpdate {
    /// Whether a newer, verified catalog was applied (only when `apply` is set).
    pub updated: bool,
    /// Whether a newer catalog is available to apply (check result).
    pub available: bool,
    /// The catalog version referred to (available/applied/current).
    pub version: u32,
    /// Number of apps in that catalog.
    pub app_count: usize,
    /// Human-readable outcome for the UI.
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

/// The catalog that ships with this build: the bundled resource copy in a
/// shipped app, otherwise the embedded copy.
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

/// Path to the verified remote catalog cache (written by `update_remote`).
fn remote_cache_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_cache_dir().ok()?;
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("curated-remote.json"))
}

fn read_remote_cache(app: &AppHandle) -> Option<CuratedFile> {
    read_curated(&remote_cache_path(app)?)
}

/// Choose the effective built-in catalog: the remote-cached one when its
/// `version` is at least the bundled one (highest version wins), else bundled.
fn pick_base(bundled: CuratedFile, remote: Option<CuratedFile>) -> CuratedFile {
    match remote {
        Some(r) if r.version >= bundled.version => r,
        _ => bundled,
    }
}

/// The source of truth for built-in apps: the repo-root file during development
/// (live edits, no rebuild), else the higher-versioned of the fetched remote
/// catalog and the bundled one. Deliberately does NOT include the per-user
/// AppData file (that holds only the user's `custom` additions).
fn load_base(app: &AppHandle) -> CuratedFile {
    // A user-supplied custom catalog replaces the official one entirely.
    if let Some(custom) = read_custom_catalog(app) {
        return custom.catalog;
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("curated.json");
    if let Some(file) = read_curated(&repo) {
        return file;
    }
    pick_base(bundled_catalog(app), read_remote_cache(app))
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

/// All display/install fields of two apps match (ignores the `custom` flag).
/// Used to tell an untouched built-in from an edited one.
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

/// Overlay the user's apps from `saved` onto the current hardcoded `base`.
/// Apps the user hasn't touched come from `base` (so built-ins refresh on
/// update). A `custom` app either **overrides** a built-in in place (same
/// source+id, keeps its category/position - used when the user edits a built-in)
/// or is **added** to its saved category when it isn't a built-in. A stale
/// removed built-in (not custom, not in base) never resurrects.
fn merge(base: CuratedFile, saved: &CuratedFile) -> CuratedFile {
    let keys = base_keys(&base);
    let mut present = keys.clone();
    let mut result = base;

    for sc in &saved.categories {
        for app in sc.apps.iter().filter(|a| a.custom) {
            let k = app_key(app);
            if keys.contains(&k) {
                // Personal override of a built-in: replace it where it already is.
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
            // Untouched built-in refreshes from base (custom=false); an edited
            // built-in or a user-added app is kept as the user's (custom=true).
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

/// Fetch the hosted catalog and verify its signature against [`CATALOG_PUBKEY`].
/// Two-phase, like the app updater: with `apply = false` it only reports whether
/// a newer catalog is *available*; with `apply = true` it caches the verified
/// bytes so `load_base` uses them. Any failure (offline, bad signature, invalid
/// JSON) leaves the current catalog untouched and returns a message for the UI.
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

    // Verify the signature BEFORE trusting or parsing the downloaded bytes.
    // `tauri signer sign` writes the .sig as base64 of the minisign signature
    // file, so decode that first (fall back to raw text for a plain minisign sig).
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

    // Compare against the catalog currently in effect (highest of bundled +
    // cached remote). A newer remote is "available"; it is only applied on
    // request, so a new version never silently replaces the current one.
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

    // Apply: persist the verified bytes so `load_base` uses them from now on.
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

/// A user-supplied catalog that replaces the official one. Holds the source it
/// came from (a file path or a URL) plus the parsed catalog itself, so the app
/// can display/refresh it without re-reading the original on every load.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomCatalog {
    /// The file path or URL the catalog was loaded from.
    pub source: String,
    /// True when `source` is a URL (vs a local file path).
    pub is_url: bool,
    pub catalog: CuratedFile,
}

/// What the UI shows about the active custom catalog (no app list).
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
    let text = std::fs::read_to_string(custom_catalog_path(app)?).ok()?;
    serde_json::from_str(&text).ok()
}

fn info_of(c: &CustomCatalog) -> CustomCatalogInfo {
    CustomCatalogInfo {
        source: c.source.clone(),
        is_url: c.is_url,
        version: c.catalog.version,
        app_count: app_count(&c.catalog),
    }
}

/// The active custom catalog's metadata, or None when the official one is in use.
pub fn custom_info(app: &AppHandle) -> Option<CustomCatalogInfo> {
    read_custom_catalog(app).as_ref().map(info_of)
}

/// Point Acy at a custom catalog (a local file or a URL). The bytes are fetched/
/// read, validated as a `CuratedFile` (the user's responsibility to format), and
/// cached; from then on `load` uses it in place of the official catalog. Any
/// failure (missing file, network error, invalid JSON) leaves the current catalog
/// untouched and returns a message for the UI. No signature check - it's the
/// user's own catalog.
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
        client
            .get(source.trim())
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Couldn't download the catalog: {e}"))?
            .bytes()
            .await
            .map_err(|e| format!("Couldn't read the catalog: {e}"))?
            .to_vec()
    } else {
        std::fs::read(source.trim()).map_err(|e| format!("Couldn't read the file: {e}"))?
    };

    let catalog: CuratedFile = serde_json::from_slice(&bytes)
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
    std::fs::write(&path, text).map_err(|e| format!("Couldn't save the catalog: {e}"))?;
    Ok(info_of(&custom))
}

/// Remove the custom catalog, reverting to the official one.
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
    fn pick_base_prefers_higher_version_remote() {
        let bundled = parse(r#"{"version":3,"categories":[]}"#);
        // Newer remote wins.
        let remote = parse(r#"{"version":5,"categories":[{"id":"x","title":"X","apps":[]}]}"#);
        assert_eq!(pick_base(bundled, Some(remote)).version, 5);

        // Equal version: remote (freshly fetched) is taken.
        let bundled = parse(r#"{"version":4,"categories":[]}"#);
        let remote = parse(r#"{"version":4,"categories":[]}"#);
        assert_eq!(pick_base(bundled, Some(remote)).version, 4);

        // Stale remote must NOT downgrade a newer bundled catalog.
        let bundled = parse(r#"{"version":6,"categories":[]}"#);
        let remote = parse(r#"{"version":2,"categories":[]}"#);
        assert_eq!(pick_base(bundled, Some(remote)).version, 6);

        // No remote cache: bundled.
        let bundled = parse(r#"{"version":6,"categories":[]}"#);
        assert_eq!(pick_base(bundled, None).version, 6);
    }

    #[test]
    fn merge_overrides_builtin_in_place() {
        // Built-in Firefox with no custom name.
        let base = parse(
            r#"{"version":2,"categories":[{"id":"browsers","title":"Browsers","apps":[
                {"id":"Mozilla.Firefox","source":"winget"},
                {"id":"Google.Chrome","source":"winget"}
            ]}]}"#,
        );
        // User edited Firefox's name - saved as a custom override (same key).
        let saved = parse(
            r#"{"version":1,"categories":[{"id":"browsers","title":"Browsers","apps":[
                {"id":"Mozilla.Firefox","source":"winget","name":"Firefox (mine)","custom":true}
            ]}]}"#,
        );
        let merged = merge(base, &saved);
        let browsers = &merged.categories[0].apps;
        // No duplicate, edit applied in place, order preserved.
        assert_eq!(browsers.len(), 2);
        assert_eq!(browsers[0].id, "Mozilla.Firefox");
        assert_eq!(browsers[0].name.as_deref(), Some("Firefox (mine)"));
        assert_eq!(browsers[1].id, "Google.Chrome");
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
