use crate::services::composer::{ComposerManager, ComposerProject, ComposerStatus};
use tauri::{command, AppHandle};

/// Get Composer status
#[command]
pub fn get_composer_status(app: AppHandle) -> Result<ComposerStatus, String> {
    ComposerManager::get_status(&app)
}

/// Install Composer
#[command]
pub async fn install_composer(app: AppHandle) -> Result<String, String> {
    ComposerManager::install(&app).await?;
    Ok("Composer installed successfully".to_string())
}

/// Uninstall Composer
#[command]
pub fn uninstall_composer(app: AppHandle) -> Result<String, String> {
    ComposerManager::uninstall(&app)?;
    Ok("Composer uninstalled successfully".to_string())
}

/// Self-update Composer
#[command]
pub fn update_composer(app: AppHandle) -> Result<String, String> {
    ComposerManager::self_update(&app)
}

/// Install project dependencies
#[command]
pub fn composer_install(app: AppHandle, project_path: String) -> Result<String, String> {
    ComposerManager::install_dependencies(&app, &project_path)
}

/// Update project dependencies
#[command]
pub fn composer_update(app: AppHandle, project_path: String) -> Result<String, String> {
    ComposerManager::update_dependencies(&app, &project_path)
}

/// Require a package
#[command]
pub fn composer_require(app: AppHandle, project_path: String, package: String, dev: bool) -> Result<String, String> {
    ComposerManager::require_package(&app, &project_path, &package, dev)
}

/// Remove a package
#[command]
pub fn composer_remove(app: AppHandle, project_path: String, package: String) -> Result<String, String> {
    ComposerManager::remove_package(&app, &project_path, &package)
}

/// Get project info
#[command]
pub fn get_composer_project(project_path: String) -> Result<ComposerProject, String> {
    ComposerManager::get_project_info(&project_path)
}

/// Run arbitrary Composer command
#[command]
pub fn composer_run(app: AppHandle, project_path: String, args: Vec<String>) -> Result<String, String> {
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    ComposerManager::run_command(&app, &project_path, &args_refs)
}
