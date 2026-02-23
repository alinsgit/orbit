use tauri::{command, AppHandle};
use crate::services::cli::{CliManager, CliStatus, BinaryUpdateInfo};

#[command]
pub fn get_cli_status(app: AppHandle) -> Result<CliStatus, String> {
    CliManager::get_status(&app)
}

#[command]
pub async fn install_cli(app: AppHandle) -> Result<String, String> {
    CliManager::install(&app).await?;
    Ok("CLI installed successfully".to_string())
}

#[command]
pub fn uninstall_cli(app: AppHandle) -> Result<String, String> {
    CliManager::uninstall(&app)?;
    Ok("CLI uninstalled".to_string())
}

#[command]
pub async fn check_cli_update(app: AppHandle) -> Result<BinaryUpdateInfo, String> {
    CliManager::check_for_update(&app).await
}

#[command]
pub async fn update_cli(app: AppHandle) -> Result<String, String> {
    CliManager::update(&app).await?;
    Ok("CLI updated successfully".to_string())
}
