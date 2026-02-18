use crate::services::backup::BackupManager;
use tauri::{command, AppHandle, Manager};

fn get_mariadb_root(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("mariadb");

    if !bin_path.exists() {
        return Err("MariaDB is not installed".to_string());
    }

    // Check if executables are in a subdirectory (e.g. mariadb/bin/)
    let bin_subdir = bin_path.join("bin");
    if bin_subdir.exists() {
        // Standard structure: mariadb/bin/mysql.exe
        // Return the parent so find_client_exe can check both paths
        return Ok(bin_path);
    }

    Ok(bin_path)
}

#[command]
pub fn export_database(
    app: AppHandle,
    database: String,
    output_path: String,
) -> Result<String, String> {
    let mariadb_root = get_mariadb_root(&app)?;
    BackupManager::export_database(&mariadb_root, &database, &output_path)
}

#[command]
pub fn export_all_databases(
    app: AppHandle,
    output_path: String,
) -> Result<String, String> {
    let mariadb_root = get_mariadb_root(&app)?;
    BackupManager::export_all_databases(&mariadb_root, &output_path)
}

#[command]
pub fn import_sql(
    app: AppHandle,
    database: String,
    sql_path: String,
) -> Result<String, String> {
    let mariadb_root = get_mariadb_root(&app)?;
    BackupManager::import_sql(&mariadb_root, &database, &sql_path)
}

#[command]
pub fn rebuild_database(
    app: AppHandle,
    database: String,
    sql_path: String,
) -> Result<String, String> {
    let mariadb_root = get_mariadb_root(&app)?;
    BackupManager::rebuild_database(&mariadb_root, &database, &sql_path)
}
