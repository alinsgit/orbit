use crate::services::templates::{TemplateInfo, TemplateManager};
use tauri::{command, AppHandle, Manager};

fn get_bin_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())
        .map(|p| p.join("bin"))
}

#[command]
pub fn list_templates(app: AppHandle) -> Result<Vec<TemplateInfo>, String> {
    let bin_path = get_bin_path(&app)?;
    TemplateManager::list_templates(&bin_path)
}

#[command]
pub fn get_template(app: AppHandle, name: String) -> Result<String, String> {
    let bin_path = get_bin_path(&app)?;
    TemplateManager::get_template(&bin_path, &name)
}

#[command]
pub fn save_template(app: AppHandle, name: String, content: String) -> Result<(), String> {
    let bin_path = get_bin_path(&app)?;
    TemplateManager::save_template(&bin_path, &name, &content)
}

#[command]
pub fn reset_template(app: AppHandle, name: String) -> Result<(), String> {
    let bin_path = get_bin_path(&app)?;
    TemplateManager::reset_template(&bin_path, &name)
}

#[command]
pub fn delete_template(app: AppHandle, name: String) -> Result<(), String> {
    let bin_path = get_bin_path(&app)?;
    TemplateManager::delete_template(&bin_path, &name)
}
