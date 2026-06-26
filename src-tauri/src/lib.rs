mod commands;
mod curated;
mod icons;
mod model;
mod runner;
mod sources;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            tray::create(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::detect_managers,
            commands::get_curated,
            commands::save_curated,
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
            commands::set_update_count,
            commands::notify,
            commands::pick_installer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
