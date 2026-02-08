mod commands;
mod services;

use services::process::ServiceManager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let service_manager = ServiceManager::new();

  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_store::Builder::default().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_sql::Builder::default().build())
    .manage(service_manager) // Register state
    .setup(|app| {
        // System Tray Setup
        let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
        let show_i = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

        // Get icon with proper error handling
        let icon = app.default_window_icon()
            .cloned()
            .ok_or_else(|| tauri::Error::AssetNotFound("window icon".to_string()))?;

        let _tray = TrayIconBuilder::with_id("orbit-tray")
            .icon(icon)
            .menu(&menu)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "quit" => {
                    // Stop all services before quitting
                    let _ = app.state::<ServiceManager>().stop_all();
                    app.exit(0);
                }
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| match event {
                TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    ..
                } => {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            })
            .build(app)?;

        // Show window on first start
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
        }
        Ok(())
    })
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            // Instead of closing, hide the window
            let _ = window.hide();
            api.prevent_close();
        }
    })
    .invoke_handler(tauri::generate_handler![
        // Service management
        commands::service::start_service,
        commands::service::stop_service,
        commands::service::reload_service,
        commands::service::get_service_status,
        commands::service::uninstall_service,
        commands::service::initialize_mariadb,
        commands::service::assign_php_port,
        commands::service::check_port_conflict,
        // Hosts file
        commands::hosts::add_host,
        commands::hosts::add_host_elevated,
        commands::hosts::remove_host,
        // Installer
        commands::installer::download_service,
        commands::installer::check_vc_redist,
        commands::versions::get_available_versions,
        commands::versions::refresh_all_versions,
        commands::scanner::get_installed_services,
        // Sites
        commands::sites::create_site,
        commands::sites::get_sites,
        commands::sites::get_site,
        commands::sites::update_site,
        commands::sites::delete_site,
        commands::sites::regenerate_site_config,
        // Nginx
        commands::sites::nginx_test_config,
        commands::sites::nginx_reload,
        commands::sites::nginx_status,
        // Export/Import
        commands::sites::export_sites,
        commands::sites::import_sites,
        // Logs
        commands::logs::get_log_files,
        commands::logs::read_log_file,
        commands::logs::clear_log_file,
        commands::logs::clear_all_logs,
        // SSL
        commands::ssl::get_ssl_status,
        commands::ssl::install_mkcert,
        commands::ssl::install_ssl_ca,
        commands::ssl::generate_ssl_cert,
        commands::ssl::get_ssl_cert,
        commands::ssl::list_ssl_certs,
        commands::ssl::delete_ssl_cert,
        // PATH management
        commands::path::add_to_path,
        commands::path::check_path_status,
        commands::path::remove_from_path,
        commands::path::add_service_to_path,
        commands::path::remove_service_from_path,
        commands::path::check_service_path_status,
        // PHP config
        commands::php_config::get_php_config,
        commands::php_config::set_php_extension,
        commands::php_config::set_php_setting,
        commands::php_config::get_php_ini_raw,
        commands::php_config::save_php_ini_raw,
        commands::php_config::configure_php_mailpit,
        commands::php_config::get_php_mailpit_status,
        commands::php_config::configure_php_redis_session,
        commands::php_config::get_php_redis_session_status,
        // PHP registry
        commands::php_registry::get_php_services,
        commands::php_registry::get_php_service,
        commands::php_registry::get_php_port,
        commands::php_registry::register_php_version,
        commands::php_registry::unregister_php_version,
        commands::php_registry::mark_php_running,
        commands::php_registry::mark_php_stopped,
        commands::php_registry::scan_php_versions,
        commands::php_registry::get_running_php_services,
        commands::php_registry::calculate_php_port,
        // Database (Adminer)
        commands::database::get_database_status,
        commands::database::get_database_tools_status,
        commands::database::install_adminer,
        commands::database::uninstall_adminer,
        commands::database::setup_adminer_nginx,
        commands::database::remove_adminer_nginx,
        // Database (PhpMyAdmin)
        commands::database::get_phpmyadmin_status,
        commands::database::install_phpmyadmin,
        commands::database::uninstall_phpmyadmin,
        commands::database::setup_phpmyadmin_nginx,
        commands::database::remove_phpmyadmin_nginx,
        // Autostart
        commands::autostart::auto_start_services,
        // Requirements
        commands::requirements::check_system_requirements,
        // Templates
        commands::templates::list_templates,
        commands::templates::get_template,
        commands::templates::save_template,
        commands::templates::reset_template,
        commands::templates::delete_template,
        // Cache (Redis)
        commands::cache::get_cache_status,
        commands::cache::install_redis,
        commands::cache::uninstall_redis,
        commands::cache::update_redis_config,
        commands::cache::get_redis_exe_path,
        // Composer
        commands::composer::get_composer_status,
        commands::composer::install_composer,
        commands::composer::uninstall_composer,
        commands::composer::update_composer,
        commands::composer::composer_install,
        commands::composer::composer_update,
        commands::composer::composer_require,
        commands::composer::composer_remove,
        commands::composer::get_composer_project,
        commands::composer::composer_run,
        // Performance
        commands::performance::get_performance_status,
        commands::performance::get_opcache_config,
        commands::performance::set_opcache_config,
        commands::performance::get_nginx_gzip_config,
        commands::performance::set_nginx_gzip_config,
        commands::performance::clear_all_caches,
        // Mailpit (Mail server)
        commands::mailpit::get_mailpit_status,
        commands::mailpit::install_mailpit,
        commands::mailpit::uninstall_mailpit,
        commands::mailpit::start_mailpit,
        commands::mailpit::stop_mailpit,
        commands::mailpit::get_mailpit_exe_path,
        // PECL Extension Manager
        commands::pecl::get_available_extensions,
        commands::pecl::install_pecl_extension,
        commands::pecl::uninstall_pecl_extension,
        commands::pecl::search_pecl_extensions
    ])
    .run(tauri::generate_context!())
    .unwrap_or_else(|e| {
        log::error!("Failed to run Tauri application: {}", e);
        std::process::exit(1);
    });
}
