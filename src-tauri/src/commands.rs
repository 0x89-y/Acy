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
            // Install-Module needs the NuGet provider and a trusted PSGallery,
            // which aren't set up on a fresh machine. Prepare them (and TLS 1.2)
            // first so the module install works without prompts.
            runner::ps_args(
                "[Net.ServicePointManager]::SecurityProtocol = \
                 [Net.ServicePointManager]::SecurityProtocol -bor 3072; \
                 Install-PackageProvider -Name NuGet -MinimumVersion 2.8.5.201 -Force \
                 -Scope CurrentUser -ErrorAction Stop; \
                 if (-not (Get-PSRepository -Name PSGallery -ErrorAction SilentlyContinue)) \
                 { Register-PSRepository -Default }; \
                 Set-PSRepository -Name PSGallery -InstallationPolicy Trusted; \
                 Install-Module Microsoft.WinGet.Client -Scope CurrentUser -Force -AllowClobber",
            ),
        ),
        Source::Msstore => {
            return Err("The Microsoft Store source is part of winget; install winget to use it.".into());
        }
        Source::Local => {
            return Err("The local source has nothing to set up; pick an installer file instead.".into());
        }
    };
    let result = runner::stream(&app, &op_id, OP_EVENT, &program, &args)
        .await
        .map_err(|e| e.to_string());
    // After a winget setup run, re-detect the PowerShell module so a freshly
    // installed one is picked up without restarting Acy.
    if matches!(source, Source::Winget) {
        sources::winget::invalidate_module_cache();
    }
    result
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

/// First whitespace token of each data row (skips a `----` separator / "Name"
/// header). Used to read bucket names out of `scoop bucket list`.
fn first_tokens(out: &str) -> Vec<String> {
    let lines: Vec<&str> = out.lines().collect();
    let body = match lines.iter().position(|l| l.trim_start().starts_with("----")) {
        Some(i) => &lines[i + 1..],
        None => &lines[..],
    };
    body.iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .filter(|t| *t != "Name")
        .map(|t| t.to_string())
        .collect()
}

/// Buckets currently added to Scoop.
#[tauri::command]
pub async fn scoop_buckets() -> Vec<String> {
    let out = runner::capture("powershell", &runner::ps_args("scoop bucket list"))
        .await
        .unwrap_or_default();
    first_tokens(&out)
}

/// Well-known buckets Scoop can add by name (main, extras, versions, …).
#[tauri::command]
pub async fn scoop_known_buckets() -> Vec<String> {
    let out = runner::capture("powershell", &runner::ps_args("scoop bucket known"))
        .await
        .unwrap_or_default();
    out.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.contains(char::is_whitespace))
        .map(|l| l.to_string())
        .collect()
}

/// Add a Scoop bucket by name. Streams output like other write operations.
#[tauri::command]
pub async fn add_scoop_bucket(app: AppHandle, name: String, op_id: String) -> Result<i32, String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("invalid bucket name".into());
    }
    let script = format!("scoop bucket add {name}");
    runner::stream(&app, &op_id, OP_EVENT, "powershell", &runner::ps_args(&script))
        .await
        .map_err(|e| e.to_string())
}

/// Refresh winget's package sources (`winget source update`). Streams output.
#[tauri::command]
pub async fn winget_update_sources(app: AppHandle, op_id: String) -> Result<i32, String> {
    runner::stream(
        &app,
        &op_id,
        OP_EVENT,
        "winget",
        &["source".to_string(), "update".to_string()],
    )
    .await
    .map_err(|e| e.to_string())
}

/// Update Scoop itself and its buckets (`scoop update`). Streams output.
#[tauri::command]
pub async fn scoop_update(app: AppHandle, op_id: String) -> Result<i32, String> {
    runner::stream(&app, &op_id, OP_EVENT, "powershell", &runner::ps_args("scoop update"))
        .await
        .map_err(|e| e.to_string())
}

/// Remove outdated Scoop app versions (`scoop cleanup *`). Streams output.
#[tauri::command]
pub async fn scoop_cleanup(app: AppHandle, op_id: String) -> Result<i32, String> {
    runner::stream(&app, &op_id, OP_EVENT, "powershell", &runner::ps_args("scoop cleanup *"))
        .await
        .map_err(|e| e.to_string())
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
