use crate::services::database::{DatabaseManager, DatabaseStatus};
use crate::services::nginx::NginxManager;
use crate::services::phpmyadmin::{PhpMyAdminManager, PhpMyAdminStatus};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Manager};

fn get_bin_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())
        .map(|p| p.join("bin"))
}

/// Combined database tools status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseToolsStatus {
    pub adminer: DatabaseStatus,
    pub phpmyadmin: PhpMyAdminStatus,
}

#[command]
pub fn get_database_status(app: AppHandle) -> Result<DatabaseStatus, String> {
    let bin_path = get_bin_path(&app)?;
    Ok(DatabaseManager::get_status(&bin_path))
}

#[command]
pub fn get_database_tools_status(app: AppHandle) -> Result<DatabaseToolsStatus, String> {
    let bin_path = get_bin_path(&app)?;
    Ok(DatabaseToolsStatus {
        adminer: DatabaseManager::get_status(&bin_path),
        phpmyadmin: PhpMyAdminManager::get_status(&bin_path),
    })
}

#[command]
pub async fn install_adminer(app: AppHandle) -> Result<String, String> {
    let bin_path = get_bin_path(&app)?;
    DatabaseManager::install(&bin_path).await
}

#[command]
pub fn uninstall_adminer(app: AppHandle) -> Result<(), String> {
    let bin_path = get_bin_path(&app)?;
    DatabaseManager::remove_nginx_config(&bin_path)?;
    DatabaseManager::uninstall(&bin_path)
}

#[command]
pub fn setup_adminer_nginx(app: AppHandle, php_port: u16) -> Result<String, String> {
    // Ensure main nginx.conf includes sites-enabled
    NginxManager::ensure_main_config(&app)?;

    let bin_path = get_bin_path(&app)?;
    DatabaseManager::create_nginx_config(&bin_path, php_port)
}

#[command]
pub fn remove_adminer_nginx(app: AppHandle) -> Result<(), String> {
    let bin_path = get_bin_path(&app)?;
    DatabaseManager::remove_nginx_config(&bin_path)
}

// PhpMyAdmin commands
#[command]
pub fn get_phpmyadmin_status(app: AppHandle) -> Result<PhpMyAdminStatus, String> {
    let bin_path = get_bin_path(&app)?;
    Ok(PhpMyAdminManager::get_status(&bin_path))
}

#[command]
pub async fn install_phpmyadmin(app: AppHandle) -> Result<String, String> {
    let bin_path = get_bin_path(&app)?;
    PhpMyAdminManager::install(&bin_path).await
}

#[command]
pub fn uninstall_phpmyadmin(app: AppHandle) -> Result<(), String> {
    let bin_path = get_bin_path(&app)?;
    PhpMyAdminManager::remove_nginx_config(&bin_path)?;
    PhpMyAdminManager::uninstall(&bin_path)
}

#[command]
pub fn setup_phpmyadmin_nginx(app: AppHandle, php_port: u16) -> Result<String, String> {
    // Ensure main nginx.conf includes sites-enabled
    NginxManager::ensure_main_config(&app)?;

    let bin_path = get_bin_path(&app)?;
    PhpMyAdminManager::create_nginx_config(&bin_path, php_port)
}

#[command]
pub fn remove_phpmyadmin_nginx(app: AppHandle) -> Result<(), String> {
    let bin_path = get_bin_path(&app)?;
    PhpMyAdminManager::remove_nginx_config(&bin_path)
}
