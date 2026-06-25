mod commands;
mod curated;
mod icons;
mod model;
mod runner;
mod sources;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::detect_managers,
            commands::get_curated,
            commands::search,
            commands::list_installed,
            commands::list_updates,
            commands::app_info,
            commands::install,
            commands::uninstall,
            commands::upgrade,
            commands::upgrade_all,
            commands::bootstrap_manager,
            commands::app_icon,
            commands::clear_icon_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
