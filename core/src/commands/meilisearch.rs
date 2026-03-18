use tauri::{command, AppHandle};
use crate::services::meilisearch::{MeilisearchManager, MeilisearchStatus};

#[command]
pub fn get_meilisearch_status(app: AppHandle) -> Result<MeilisearchStatus, String> {
    MeilisearchManager::get_status(&app)
}

#[command]
pub async fn install_meilisearch(app: AppHandle) -> Result<String, String> {
    MeilisearchManager::install(&app).await?;
    Ok("Meilisearch installed successfully".to_string())
}

#[command]
pub fn uninstall_meilisearch(app: AppHandle) -> Result<String, String> {
    MeilisearchManager::stop().ok();
    MeilisearchManager::uninstall(&app)?;
    Ok("Meilisearch uninstalled".to_string())
}

#[command]
pub fn start_meilisearch(app: AppHandle) -> Result<String, String> {
    MeilisearchManager::start(&app)?;
    Ok("Meilisearch started".to_string())
}

#[command]
pub fn stop_meilisearch() -> Result<String, String> {
    MeilisearchManager::stop()?;
    Ok("Meilisearch stopped".to_string())
}

#[command]
pub fn get_meilisearch_exe_path(app: AppHandle) -> Result<String, String> {
    let path = MeilisearchManager::get_exe_path(&app)?;
    Ok(path.to_string_lossy().to_string())
}
