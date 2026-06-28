mod commands;
mod curated;
mod icons;
mod model;
mod runner;
mod sources;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
    }

    builder
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
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
            commands::scoop_buckets,
            commands::scoop_known_buckets,
            commands::add_scoop_bucket,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
