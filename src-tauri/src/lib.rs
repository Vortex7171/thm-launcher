mod accounts; mod instances; mod java; mod launch; mod mc; mod misc; mod mods; mod net; mod store; mod thm;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            store::DIR.set(dir).ok();
            if let Some(w) = app.get_webview_window("main") { w.set_theme(Some(tauri::Theme::Dark)).ok(); }
            instances::ensure_default().ok();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            accounts::list_accounts, accounts::add_offline, accounts::remove_account, accounts::set_active_account, accounts::ms_begin, accounts::ms_poll,
            instances::list_instances, instances::list_mc_versions, instances::create_instance, instances::rename_instance, instances::update_instance,
            instances::delete_instance, instances::clone_instance, instances::repair_instance, instances::export_instance, instances::import_instance,
            launch::install_instance, launch::launch_instance, launch::kill_instance, launch::list_running,
            mods::analyze_mods, mods::add_mod_jar, mods::toggle_mod, mods::delete_mod, mods::export_mods,
            mods::modrinth_search, mods::modrinth_install, mods::check_mod_updates, mods::apply_mod_update,
            thm::thm_manifest, thm::refresh_manifest, thm::thm_slots, thm::thm_install_slot,
            misc::get_settings, misc::save_settings, misc::list_activity, misc::launcher_version, misc::server_status,
            misc::announcements, misc::check_updates, misc::apply_loader_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running THM Launcher");
}
