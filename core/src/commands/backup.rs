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

    Ok(bin_path)
}

fn get_pg_root(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let bin_base = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    // PostgreSQL may be in bin/postgresql/ or bin/postgresql/pgsql/
    let pg_path = bin_base.join("postgresql");
    if !pg_path.exists() {
        return Err("PostgreSQL is not installed".to_string());
    }

    // Check for nested pgsql/ structure
    let nested = pg_path.join("pgsql");
    if nested.join("bin").exists() {
        return Ok(nested);
    }
    if pg_path.join("bin").exists() {
        return Ok(pg_path);
    }

    Err("PostgreSQL bin directory not found".to_string())
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

// ─── PostgreSQL Backup Commands ─────────────────────────────

#[command]
pub fn pg_export_database(
    app: AppHandle,
    database: String,
    output_path: String,
) -> Result<String, String> {
    let pg_root = get_pg_root(&app)?;
    BackupManager::pg_export_database(&pg_root, &database, &output_path)
}

#[command]
pub fn pg_import_sql(
    app: AppHandle,
    database: String,
    sql_path: String,
) -> Result<String, String> {
    let pg_root = get_pg_root(&app)?;
    BackupManager::pg_import_sql(&pg_root, &database, &sql_path)
}

// ─── MariaDB Rebuild ───────────────────────────────────────

#[command]
pub fn rebuild_database(
    app: AppHandle,
    database: String,
    sql_path: String,
) -> Result<String, String> {
    let mariadb_root = get_mariadb_root(&app)?;
    BackupManager::rebuild_database(&mariadb_root, &database, &sql_path)
}
