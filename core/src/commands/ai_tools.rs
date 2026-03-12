use crate::services::ai_tools::{AiToolStatus, ClaudeCodeManager, GeminiCliManager};
use tauri::{command, AppHandle, Manager};

/// Get Claude Code status
#[command]
pub fn get_claude_code_status(app: AppHandle) -> Result<AiToolStatus, String> {
    ClaudeCodeManager::get_status(&app)
}

/// Install Claude Code
#[command]
pub fn install_claude_code(app: AppHandle) -> Result<String, String> {
    ClaudeCodeManager::install(&app)?;
    Ok("Claude Code installed successfully".to_string())
}

/// Uninstall Claude Code
#[command]
pub fn uninstall_claude_code(app: AppHandle) -> Result<String, String> {
    ClaudeCodeManager::uninstall(&app)?;
    Ok("Claude Code uninstalled successfully".to_string())
}

/// Update Claude Code
#[command]
pub fn update_claude_code(app: AppHandle) -> Result<String, String> {
    ClaudeCodeManager::update(&app)?;
    Ok("Claude Code updated successfully".to_string())
}

/// Get Gemini CLI status
#[command]
pub fn get_gemini_cli_status(app: AppHandle) -> Result<AiToolStatus, String> {
    GeminiCliManager::get_status(&app)
}

/// Install Gemini CLI
#[command]
pub fn install_gemini_cli(app: AppHandle) -> Result<String, String> {
    GeminiCliManager::install(&app)?;
    Ok("Gemini CLI installed successfully".to_string())
}

/// Uninstall Gemini CLI
#[command]
pub fn uninstall_gemini_cli(app: AppHandle) -> Result<String, String> {
    GeminiCliManager::uninstall(&app)?;
    Ok("Gemini CLI uninstalled successfully".to_string())
}

/// Update Gemini CLI
#[command]
pub fn update_gemini_cli(app: AppHandle) -> Result<String, String> {
    GeminiCliManager::update(&app)?;
    Ok("Gemini CLI updated successfully".to_string())
}

/// Generate and write AI context files for a site project
#[command]
pub fn generate_ai_context_cmd(app: AppHandle, domain: String) -> Result<String, String> {
    let data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    crate::services::ai_tools::write_context_file(&app, &domain, &data_dir)
}
