use crate::services::ai_tools::{AiToolStatus, ClaudeCodeManager, GeminiCliManager};
use tauri::{command, AppHandle, Manager};

/// Get Claude Code status
#[command]
pub fn get_claude_code_status(app: AppHandle) -> Result<AiToolStatus, String> {
    ClaudeCodeManager::get_status(&app)
}

/// Install Claude Code (native installer)
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

/// Generate and write AI context files for a site project
#[command]
pub fn generate_ai_context_cmd(app: AppHandle, domain: String) -> Result<String, String> {
    let data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    crate::services::ai_tools::write_context_file(&app, &domain, &data_dir)
}

/// Register orbit-mcp in Claude Code's MCP configuration (~/.claude.json)
#[command]
pub fn setup_mcp_config(app: AppHandle) -> Result<String, String> {
    crate::services::ai_tools::setup_mcp_for_claude(&app)?;
    Ok("MCP config updated for Claude Code".to_string())
}

/// Open a project in the OS native terminal with an AI tool (claude/gemini)
#[command]
pub fn open_in_terminal(app: AppHandle, tool: String, project_path: String, domain: Option<String>) -> Result<String, String> {
    // Generate context if domain provided
    if let Some(d) = &domain {
        let data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        crate::services::ai_tools::write_context_file(&app, d, &data_dir).ok();
    }

    let tool_cmd = match tool.as_str() {
        "claude-code" => "claude",
        "gemini-cli" => "gemini",
        _ => return Err(format!("Unknown tool: {tool}")),
    };

    // Build PATH with Orbit service binaries
    let orbit_path = crate::services::terminal::build_orbit_path(&app);

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // Check if Windows Terminal (wt.exe) is available
        let has_wt = crate::services::hidden_command("where")
            .args(["wt.exe"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_wt {
            Command::new("cmd")
                .env("PATH", &orbit_path)
                .args(["/c", "start", "", "wt.exe", "-d", &project_path, "cmd", "/k", tool_cmd])
                .spawn()
                .map_err(|e| format!("Failed to open Windows Terminal: {e}"))?;
            Ok("Opened in Windows Terminal".to_string())
        } else {
            let run = format!("cd /d \"{}\" && {}", project_path, tool_cmd);
            Command::new("cmd")
                .env("PATH", &orbit_path)
                .args(["/c", "start", "", "cmd.exe", "/k", &run])
                .spawn()
                .map_err(|e| format!("Failed to open terminal: {e}"))?;
            Ok("Opened in Command Prompt".to_string())
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let escaped_path = project_path.replace('\'', "'\\''");
        let script = format!(
            "tell application \"Terminal\" to do script \"cd '{}' && {}\"",
            escaped_path, tool_cmd
        );
        Command::new("osascript")
            .env("PATH", &orbit_path)
            .args(["-e", &script])
            .spawn()
            .map_err(|e| format!("Failed to open Terminal.app: {e}"))?;
        Ok("Opened in Terminal.app".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let terminals: &[(&str, &[&str])] = &[
            ("gnome-terminal", &["--working-directory", &project_path, "--", tool_cmd]),
            ("konsole", &["--workdir", &project_path, "-e", tool_cmd]),
            ("xfce4-terminal", &["--working-directory", &project_path, "-e", tool_cmd]),
            ("alacritty", &["--working-directory", &project_path, "-e", tool_cmd]),
            ("kitty", &["--directory", &project_path, tool_cmd]),
        ];

        for (term, args) in terminals {
            let found = Command::new("which")
                .arg(term)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if found {
                Command::new(term)
                    .env("PATH", &orbit_path)
                    .args(*args)
                    .spawn()
                    .map_err(|e| format!("Failed to open {term}: {e}"))?;
                return Ok(format!("Opened in {term}"));
            }
        }

        let run = format!("cd '{}' && {}", project_path.replace('\'', "'\\''"), tool_cmd);
        Command::new("xterm")
            .env("PATH", &orbit_path)
            .args(["-e", &run])
            .spawn()
            .map_err(|e| format!("No suitable terminal found. Last tried xterm: {e}"))?;
        Ok("Opened in xterm".to_string())
    }
}
