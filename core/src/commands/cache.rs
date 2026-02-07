use crate::services::cache::{CacheConfig, CacheManager, CacheStatus};
use crate::services::download::{download_file, extract_zip};
use tauri::{command, AppHandle, Manager};
use std::fs;

/// Get cache services status
#[command]
pub fn get_cache_status(app: AppHandle) -> Result<CacheStatus, String> {
    CacheManager::get_status(&app)
}

/// Install Redis
#[command]
pub async fn install_redis(app: AppHandle) -> Result<String, String> {
    let bin_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    let redis_dir = bin_dir.join("redis");
    if redis_dir.exists() {
        return Err("Redis is already installed".to_string());
    }

    // Create directory
    fs::create_dir_all(&redis_dir)
        .map_err(|e| format!("Failed to create Redis directory: {}", e))?;

    // Download Redis from registry
    let url = CacheManager::get_redis_download_url().await;
    let zip_path = bin_dir.join("redis.zip");

    download_file(&url, &zip_path).await?;

    // Extract
    extract_zip(&zip_path, &redis_dir)?;

    // Clean up zip
    let _ = fs::remove_file(&zip_path);

    // Create default config
    let config = CacheConfig {
        port: 6379,
        max_memory: "128mb".to_string(),
        enabled: true,
    };
    CacheManager::create_redis_config(&app, &config)?;

    Ok("Redis installed successfully".to_string())
}

/// Uninstall Redis
#[command]
pub fn uninstall_redis(app: AppHandle) -> Result<String, String> {
    let bin_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    let redis_dir = bin_dir.join("redis");
    if !redis_dir.exists() {
        return Err("Redis is not installed".to_string());
    }

    fs::remove_dir_all(&redis_dir)
        .map_err(|e| format!("Failed to remove Redis: {}", e))?;

    Ok("Redis uninstalled successfully".to_string())
}

/// Update Redis configuration
#[command]
pub fn update_redis_config(app: AppHandle, port: u16, max_memory: String) -> Result<String, String> {
    let config = CacheConfig {
        port,
        max_memory,
        enabled: true,
    };
    CacheManager::create_redis_config(&app, &config)?;
    Ok("Redis configuration updated".to_string())
}

/// Get Redis executable path for service manager
#[command]
pub fn get_redis_exe_path(app: AppHandle) -> Result<String, String> {
    let bin_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    let redis_exe = bin_dir.join("redis").join("redis-server.exe");
    if !redis_exe.exists() {
        return Err("Redis is not installed".to_string());
    }

    Ok(redis_exe.to_string_lossy().to_string())
}
