use tauri::{command, AppHandle};
use crate::services::cli::{CliManager, CliStatus};

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
