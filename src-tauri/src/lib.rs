mod arp;
mod commands;
mod curated;
mod icons;
mod model;
mod runner;
mod sources;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }));
    }

    builder = builder
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
            commands::update_curated_catalog,
            commands::custom_catalog_info,
            commands::set_custom_catalog,
            commands::clear_custom_catalog,
            commands::pick_catalog_file,
            commands::search,
            commands::list_installed,
            commands::list_installed_fast,
            commands::list_updates,
            commands::app_info,
            commands::install,
            commands::scoop_needed_bucket,
            commands::uninstall,
            commands::upgrade,
            commands::bootstrap_manager,
            commands::app_icon,
            commands::clear_icon_cache,
            commands::refetch_missing_icons,
            commands::set_update_count,
            commands::notify,
            commands::pick_installer,
            commands::scoop_buckets,
            commands::scoop_known_buckets,
            commands::add_scoop_bucket,
            commands::remove_scoop_bucket,
            commands::winget_update_sources,
            commands::scoop_update,
            commands::scoop_cleanup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
