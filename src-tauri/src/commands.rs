use crate::curated::{self, CuratedFile};
use crate::model::{ManagerStatus, Package, SearchHit, Source};
use crate::runner;
use crate::sources::{self, merge};
use crate::tray::TRAY_ID;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
use tokio::task::JoinSet;

/// Event channel for streamed install/uninstall/upgrade output.
const OP_EVENT: &str = "op-log";

/// Detect which managers are usable on this machine.
#[tauri::command]
pub async fn detect_managers() -> Vec<ManagerStatus> {
    let mut set = JoinSet::new();
    for src in sources::all() {
        set.spawn(async move { src.status().await });
    }
    let mut out = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(status) = res {
            out.push(status);
        }
    }
    out.sort_by_key(|m| m.source.priority());
    out
}

/// The curated catalog (categories + apps).
#[tauri::command]
pub async fn get_curated(app: AppHandle) -> CuratedFile {
    curated::load(&app)
}

/// Persist an edited curated catalog to the per-user config dir.
#[tauri::command]
pub async fn save_curated(app: AppHandle, file: CuratedFile) -> Result<(), String> {
    curated::save(&app, &file).map(|_| ()).map_err(|e| e.to_string())
}

/// Search the given managers and return merged, de-duplicated hits.
#[tauri::command]
pub async fn search(query: String, sources: Vec<Source>) -> Result<Vec<SearchHit>, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let mut set = JoinSet::new();
    for source in sources {
        let src = sources::for_source(source);
        let q = query.clone();
        set.spawn(async move { src.search(&q).await.unwrap_or_default() });
    }

    let mut all = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(found) = res {
            all.extend(found);
        }
    }
    Ok(merge(all))
}

/// Everything installed across the given managers.
#[tauri::command]
pub async fn list_installed(sources: Vec<Source>) -> Result<Vec<Package>, String> {
    let mut set = JoinSet::new();
    for source in sources {
        let src = sources::for_source(source);
        set.spawn(async move { src.list_installed().await.unwrap_or_default() });
    }
    let mut all = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(found) = res {
            all.extend(found);
        }
    }
    all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(all)
}

/// All available updates across the given managers.
#[tauri::command]
pub async fn list_updates(sources: Vec<Source>) -> Result<Vec<Package>, String> {
    let mut set = JoinSet::new();
    for source in sources {
        let src = sources::for_source(source);
        set.spawn(async move { src.list_updates().await.unwrap_or_default() });
    }
    let mut all = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(found) = res {
            all.extend(found);
        }
    }
    all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(all)
}

/// Rich details for a single package.
#[tauri::command]
pub async fn app_info(source: Source, id: String) -> Result<Option<Package>, String> {
    sources::for_source(source)
        .info(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install(
    app: AppHandle,
    source: Source,
    id: String,
    op_id: String,
) -> Result<i32, String> {
    let (program, args) = sources::for_source(source).install_cmd(&id);
    runner::stream(&app, &op_id, OP_EVENT, &program, &args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn uninstall(
    app: AppHandle,
    source: Source,
    id: String,
    op_id: String,
) -> Result<i32, String> {
    let (program, args) = sources::for_source(source).uninstall_cmd(&id);
    runner::stream(&app, &op_id, OP_EVENT, &program, &args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upgrade(
    app: AppHandle,
    source: Source,
    id: String,
    op_id: String,
) -> Result<i32, String> {
    let (program, args) = sources::for_source(source).upgrade_cmd(&id);
    runner::stream(&app, &op_id, OP_EVENT, &program, &args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upgrade_all(app: AppHandle, source: Source, op_id: String) -> Result<i32, String> {
    let (program, args) = sources::for_source(source).upgrade_all_cmd();
    runner::stream(&app, &op_id, OP_EVENT, &program, &args)
        .await
        .map_err(|e| e.to_string())
}

/// Install a missing manager (or the winget PowerShell module). Streams output.
#[tauri::command]
pub async fn bootstrap_manager(
    app: AppHandle,
    source: Source,
    op_id: String,
) -> Result<i32, String> {
    let (program, args) = match source {
        Source::Scoop => (
            "powershell".to_string(),
            runner::ps_args(
                "Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force; \
                 Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression",
            ),
        ),
        Source::Choco => (
            "powershell".to_string(),
            runner::ps_args(
                "Set-ExecutionPolicy Bypass -Scope Process -Force; \
                 [System.Net.ServicePointManager]::SecurityProtocol = 3072; \
                 Invoke-Expression ((New-Object System.Net.WebClient).DownloadString(\
                 'https://community.chocolatey.org/install.ps1'))",
            ),
        ),
        Source::Winget => (
            "powershell".to_string(),
            runner::ps_args(
                "Install-Module Microsoft.WinGet.Client -Scope CurrentUser -Force -AllowClobber",
            ),
        ),
        Source::Msstore => {
            return Err("The Microsoft Store source is part of winget; install winget to use it.".into());
        }
        Source::Local => {
            return Err("The local source has nothing to set up; pick an installer file instead.".into());
        }
    };
    runner::stream(&app, &op_id, OP_EVENT, &program, &args)
        .await
        .map_err(|e| e.to_string())
}

/// Reflect the available-update count in the tray tooltip, so it stays visible
/// even when the window is hidden to the tray.
#[tauri::command]
pub async fn set_update_count(app: AppHandle, count: u32) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let tip = match count {
            0 => "Acy".to_string(),
            1 => "Acy — 1 update available".to_string(),
            n => format!("Acy — {n} updates available"),
        };
        let _ = tray.set_tooltip(Some(&tip));
    }
}

/// Show a desktop notification (used for background update alerts when enabled).
#[tauri::command]
pub async fn notify(app: AppHandle, title: String, body: String) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Open a file picker for a local/network installer, returning the chosen path
/// (or null if cancelled). Used by the "local" install source.
#[tauri::command]
pub async fn pick_installer(app: AppHandle) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Installer", &["exe", "msi"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    rx.await
        .ok()
        .flatten()
        .and_then(|fp| fp.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Cached or freshly-fetched icon (base64 data URL) for a package, or null.
#[tauri::command]
pub async fn app_icon(
    app: AppHandle,
    source: Source,
    id: String,
    homepage: Option<String>,
) -> Option<String> {
    crate::icons::get_icon(&app, source, id, homepage).await
}

/// Delete all cached icons.
#[tauri::command]
pub async fn clear_icon_cache(app: AppHandle) -> Result<(), String> {
    crate::icons::clear_cache(&app).map_err(|e| e.to_string())
}
