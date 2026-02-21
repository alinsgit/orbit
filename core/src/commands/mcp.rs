use tauri::{command, AppHandle};
use crate::services::mcp::{McpManager, McpStatus};

#[command]
pub fn get_mcp_status(app: AppHandle) -> Result<McpStatus, String> {
    McpManager::get_status(&app)
}

#[command]
pub async fn install_mcp(app: AppHandle) -> Result<String, String> {
    McpManager::install(&app).await?;
    Ok("MCP server installed successfully".to_string())
}

#[command]
pub fn uninstall_mcp(app: AppHandle) -> Result<String, String> {
    McpManager::stop().ok(); // Stop first if running
    McpManager::uninstall(&app)?;
    Ok("MCP server uninstalled".to_string())
}

#[command]
pub fn start_mcp(app: AppHandle) -> Result<String, String> {
    McpManager::start(&app)?;
    Ok("MCP server started".to_string())
}

#[command]
pub fn stop_mcp() -> Result<String, String> {
    McpManager::stop()?;
    Ok("MCP server stopped".to_string())
}

#[command]
pub fn get_mcp_binary_path(app: AppHandle) -> Result<String, String> {
    let path = McpManager::get_exe_path(&app)?;
    Ok(path.to_string_lossy().to_string())
}
