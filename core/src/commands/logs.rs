use crate::services::logs::{LogEntry, LogFile, LogManager};
use crate::services::validation::validate_log_path;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn get_log_files(app: AppHandle) -> Result<Vec<LogFile>, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    LogManager::get_log_files(&bin_path)
}

/// Get allowed base path for log files
fn get_allowed_log_base(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))
}

#[command]
pub fn read_log_file(app: AppHandle, path: String, lines: usize, offset: usize) -> Result<Vec<LogEntry>, String> {
    // Validate path is within allowed directory
    let allowed_base = get_allowed_log_base(&app)?;
    let validated_path = validate_log_path(&path, &allowed_base)
        .map_err(|e| e.to_string())?;

    LogManager::read_log(&validated_path.to_string_lossy(), lines, offset)
}

#[command]
pub fn clear_log_file(app: AppHandle, path: String) -> Result<(), String> {
    // Validate path is within allowed directory
    let allowed_base = get_allowed_log_base(&app)?;
    let validated_path = validate_log_path(&path, &allowed_base)
        .map_err(|e| e.to_string())?;

    LogManager::clear_log(&validated_path.to_string_lossy())
}

/// Clear all log files
#[command]
pub fn clear_all_logs(app: AppHandle) -> Result<usize, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    let log_files = LogManager::get_log_files(&bin_path)?;
    let mut cleared = 0;

    for log_file in log_files {
        if LogManager::clear_log(&log_file.path).is_ok() {
            cleared += 1;
        }
    }

    Ok(cleared)
}
