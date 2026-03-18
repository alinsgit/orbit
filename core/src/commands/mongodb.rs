use crate::services::mongodb::MongoDBManager;
use tauri::{command, AppHandle, Manager};

fn get_bin_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())
        .map(|p| p.join("bin"))
}

#[command]
pub fn mongo_list_databases(app: AppHandle) -> Result<Vec<String>, String> {
    let bin_dir = get_bin_dir(&app)?;
    MongoDBManager::list_databases(&bin_dir)
}

#[command]
pub fn mongo_list_collections(app: AppHandle, database: String) -> Result<Vec<String>, String> {
    let bin_dir = get_bin_dir(&app)?;
    MongoDBManager::list_collections(&bin_dir, &database)
}

#[command]
pub fn mongo_db_stats(app: AppHandle, database: String) -> Result<String, String> {
    let bin_dir = get_bin_dir(&app)?;
    MongoDBManager::get_db_stats(&bin_dir, &database)
}

#[command]
pub fn mongo_drop_database(app: AppHandle, database: String) -> Result<String, String> {
    let bin_dir = get_bin_dir(&app)?;
    MongoDBManager::drop_database(&bin_dir, &database)
}

#[command]
pub fn mongo_run_command(app: AppHandle, database: String, js_command: String) -> Result<String, String> {
    let bin_dir = get_bin_dir(&app)?;
    MongoDBManager::run_command(&bin_dir, &database, &js_command)
}
