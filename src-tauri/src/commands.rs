use crate::curated::{self, CuratedFile};
use crate::model::{ManagerStatus, Package, SearchHit, Source};
use crate::runner;
use crate::sources::{self, merge};
use crate::tray::TRAY_ID;
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
use tokio::task::JoinSet;

const OP_EVENT: &str = "op-log";

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

#[tauri::command]
pub async fn get_curated(app: AppHandle) -> CuratedFile {
    curated::load(&app)
}

#[tauri::command]
pub async fn save_curated(app: AppHandle, file: CuratedFile) -> Result<(), String> {
    curated::save(&app, &file).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_curated_catalog(
    app: AppHandle,
    apply: bool,
) -> Result<curated::CatalogUpdate, String> {
    curated::update_remote(&app, apply).await
}

#[tauri::command]
pub async fn custom_catalog_info(app: AppHandle) -> Option<curated::CustomCatalogInfo> {
    curated::custom_info(&app)
}

#[tauri::command]
pub async fn set_custom_catalog(
    app: AppHandle,
    source: String,
    is_url: bool,
) -> Result<curated::CustomCatalogInfo, String> {
    curated::set_custom(&app, source, is_url).await
}

#[tauri::command]
pub async fn clear_custom_catalog(app: AppHandle) -> Result<(), String> {
    curated::clear_custom(&app)
}

#[tauri::command]
pub async fn pick_catalog_file(app: AppHandle) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Catalog", &["json"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    rx.await
        .ok()
        .flatten()
        .and_then(|fp| fp.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

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

#[tauri::command]
pub async fn list_installed(sources: Vec<Source>) -> Result<Vec<Package>, String> {
    let mut set = JoinSet::new();
    for source in sources {
        let src = sources::for_source(source);
        set.spawn(async move { src.list_installed().await.map_err(|e| e.to_string()) });
    }
    let (all, errored) = collect(&mut set).await;
    if all.is_empty() && errored {
        return Err(
            "Couldn't read installed apps (a package manager timed out or failed).".into(),
        );
    }
    Ok(all)
}

async fn collect(set: &mut JoinSet<Result<Vec<Package>, String>>) -> (Vec<Package>, bool) {
    let mut all = Vec::new();
    let mut errored = false;
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(found)) => all.extend(found),
            Ok(Err(_)) | Err(_) => errored = true,
        }
    }
    all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    (all, errored)
}

#[tauri::command]
pub async fn list_installed_fast() -> Result<Vec<Package>, String> {
    tokio::task::spawn_blocking(crate::arp::list_installed)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_updates(sources: Vec<Source>) -> Result<Vec<Package>, String> {
    let mut set = JoinSet::new();
    for source in sources {
        let src = sources::for_source(source);
        set.spawn(async move { src.list_updates().await.map_err(|e| e.to_string()) });
    }
    let (all, errored) = collect(&mut set).await;
    if all.is_empty() && errored {
        return Err("Couldn't check for updates (a package manager timed out or failed).".into());
    }
    Ok(all)
}

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
pub async fn scoop_needed_bucket(id: String) -> Option<String> {
    sources::scoop::needed_bucket(&id)
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
    if matches!(source, Source::Winget) {
        sources::winget::invalidate_module_cache();
    }
    result
}

#[tauri::command]
pub async fn set_update_count(app: AppHandle, count: u32) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let tip = match count {
            0 => "Acy".to_string(),
            1 => "Acy - 1 update available".to_string(),
            n => format!("Acy - {n} updates available"),
        };
        let _ = tray.set_tooltip(Some(&tip));
    }
}

#[tauri::command]
pub async fn notify(app: AppHandle, title: String, body: String) {
    let _ = app.notification().builder().title(title).body(body).show();
}

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

#[tauri::command]
pub async fn scoop_buckets() -> Vec<String> {
    let out = runner::capture("powershell", &runner::ps_args("scoop bucket list"))
        .await
        .unwrap_or_default();
    first_tokens(&out)
}

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

#[tauri::command]
pub async fn remove_scoop_bucket(
    app: AppHandle,
    name: String,
    op_id: String,
) -> Result<i32, String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("invalid bucket name".into());
    }
    let script = format!("scoop bucket rm {name}");
    runner::stream(&app, &op_id, OP_EVENT, "powershell", &runner::ps_args(&script))
        .await
        .map_err(|e| e.to_string())
}

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

#[tauri::command]
pub async fn scoop_update(app: AppHandle, op_id: String) -> Result<i32, String> {
    runner::stream(&app, &op_id, OP_EVENT, "powershell", &runner::ps_args("scoop update"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scoop_cleanup(app: AppHandle, op_id: String) -> Result<i32, String> {
    runner::stream(&app, &op_id, OP_EVENT, "powershell", &runner::ps_args("scoop cleanup *"))
        .await
        .map_err(|e| e.to_string())
}

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

#[tauri::command]
pub async fn app_icon(
    app: AppHandle,
    source: Source,
    id: String,
    homepage: Option<String>,
    steamgrid_key: Option<String>,
    game_name: Option<String>,
) -> Option<String> {
    crate::icons::get_icon(&app, source, id, homepage, steamgrid_key, game_name).await
}

#[tauri::command]
pub async fn clear_icon_cache(app: AppHandle) -> Result<(), String> {
    crate::icons::clear_cache(&app).map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconItem {
    source: Source,
    id: String,
    homepage: Option<String>,
    game_name: Option<String>,
}

#[derive(serde::Serialize)]
pub struct IconRefetch {
    fetched: u32,
    failed: u32,
}

#[derive(Clone, serde::Serialize)]
struct IconRefetchProgress {
    current: usize,
    total: usize,
}

const ICON_PROGRESS_EVENT: &str = "icon-refetch-progress";

#[tauri::command]
pub async fn refetch_missing_icons(
    app: AppHandle,
    items: Vec<IconItem>,
    steamgrid_key: Option<String>,
) -> IconRefetch {
    let todo: Vec<IconItem> = items
        .into_iter()
        .filter(|it| !crate::icons::has_icon(&app, it.source, &it.id))
        .collect();
    let total = todo.len();

    let mut fetched = 0;
    let mut failed = 0;
    for (i, item) in todo.into_iter().enumerate() {
        let _ = app.emit(ICON_PROGRESS_EVENT, IconRefetchProgress { current: i, total });
        crate::icons::clear_miss_marker(&app, item.source, &item.id);
        match crate::icons::get_icon(
            &app,
            item.source,
            item.id,
            item.homepage,
            steamgrid_key.clone(),
            item.game_name,
        )
        .await
        {
            Some(_) => fetched += 1,
            None => failed += 1,
        }
    }
    let _ = app.emit(
        ICON_PROGRESS_EVENT,
        IconRefetchProgress {
            current: total,
            total,
        },
    );
    IconRefetch { fetched, failed }
}
