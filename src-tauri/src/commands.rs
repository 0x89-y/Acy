use crate::curated::{self, CuratedFile};
use crate::model::{ManagerStatus, Package, SearchHit, Source};
use crate::runner;
use crate::sources::{self, merge};
use tauri::AppHandle;
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
    };
    runner::stream(&app, &op_id, OP_EVENT, &program, &args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn app_icon(
    app: AppHandle,
    source: Source,
    id: String,
    homepage: Option<String>,
) -> Option<String> {
    crate::icons::get_icon(&app, source, id, homepage).await
}

#[tauri::command]
pub async fn clear_icon_cache(app: AppHandle) -> Result<(), String> {
    crate::icons::clear_cache(&app).map_err(|e| e.to_string())
}
