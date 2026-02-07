use tauri::{command, AppHandle};
use crate::services::mailpit::{MailpitManager, MailpitStatus};

#[command]
pub fn get_mailpit_status(app: AppHandle) -> Result<MailpitStatus, String> {
    MailpitManager::get_status(&app)
}

#[command]
pub async fn install_mailpit(app: AppHandle) -> Result<String, String> {
    MailpitManager::install(&app).await?;
    Ok("Mailpit installed successfully".to_string())
}

#[command]
pub fn uninstall_mailpit(app: AppHandle) -> Result<String, String> {
    MailpitManager::stop().ok(); // Stop first if running
    MailpitManager::uninstall(&app)?;
    Ok("Mailpit uninstalled".to_string())
}

#[command]
pub fn start_mailpit(app: AppHandle) -> Result<String, String> {
    MailpitManager::start(&app)?;
    Ok("Mailpit started".to_string())
}

#[command]
pub fn stop_mailpit() -> Result<String, String> {
    MailpitManager::stop()?;
    Ok("Mailpit stopped".to_string())
}

#[command]
pub fn get_mailpit_exe_path(app: AppHandle) -> Result<String, String> {
    let path = MailpitManager::get_exe_path(&app)?;
    Ok(path.to_string_lossy().to_string())
}
