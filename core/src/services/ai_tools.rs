use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::hidden_command;

/// AI tool status information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiToolStatus {
  pub installed: bool,
  pub path: Option<String>,
  pub version: Option<String>,
}

impl Default for AiToolStatus {
  fn default() -> Self {
    Self {
      installed: false,
      path: None,
      version: None,
    }
  }
}

pub struct ClaudeCodeManager;

impl ClaudeCodeManager {
  /// Get npm executable path
  pub fn get_npm_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
      .path()
      .app_local_data_dir()
      .map_err(|e| e.to_string())?
      .join("bin")
      .join("nodejs");

    #[cfg(target_os = "windows")]
    return Ok(base.join("npm.cmd"));

    #[cfg(not(target_os = "windows"))]
    return Ok(base.join("bin").join("npm"));
  }

  /// Get claude executable path
  pub fn get_exe_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
      .path()
      .app_local_data_dir()
      .map_err(|e| e.to_string())?
      .join("bin")
      .join("nodejs");

    #[cfg(target_os = "windows")]
    return Ok(base.join("claude.cmd"));

    #[cfg(not(target_os = "windows"))]
    return Ok(base.join("bin").join("claude"));
  }

  /// Check if Claude Code is installed
  pub fn is_installed(app: &AppHandle) -> Result<bool, String> {
    Ok(Self::get_exe_path(app)?.exists())
  }

  /// Get Claude Code version
  pub fn get_version(app: &AppHandle) -> Result<Option<String>, String> {
    if !Self::is_installed(app)? {
      return Ok(None);
    }

    let exe = Self::get_exe_path(app)?;
    let output = hidden_command(&exe)
      .args(["--version"])
      .output()
      .ok();

    if let Some(output) = output {
      if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let version = stdout.trim().to_string();
        if !version.is_empty() {
          return Ok(Some(version));
        }
      }
    }

    Ok(None)
  }

  /// Get full Claude Code status
  pub fn get_status(app: &AppHandle) -> Result<AiToolStatus, String> {
    let installed = Self::is_installed(app)?;
    let path = if installed {
      Some(Self::get_exe_path(app)?.to_string_lossy().to_string())
    } else {
      None
    };
    let version = if installed {
      Self::get_version(app)?
    } else {
      None
    };

    Ok(AiToolStatus {
      installed,
      path,
      version,
    })
  }

  /// Install Claude Code via npm
  pub fn install(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed. Please install Node.js first.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["install", "-g", "@anthropic-ai/claude-code"])
      .output()
      .map_err(|e| format!("Failed to run npm install: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }

  /// Uninstall Claude Code via npm
  pub fn uninstall(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["uninstall", "-g", "@anthropic-ai/claude-code"])
      .output()
      .map_err(|e| format!("Failed to run npm uninstall: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }

  /// Update Claude Code via npm
  pub fn update(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["update", "-g", "@anthropic-ai/claude-code"])
      .output()
      .map_err(|e| format!("Failed to run npm update: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }
}

pub struct GeminiCliManager;

impl GeminiCliManager {
  /// Get npm executable path
  pub fn get_npm_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
      .path()
      .app_local_data_dir()
      .map_err(|e| e.to_string())?
      .join("bin")
      .join("nodejs");

    #[cfg(target_os = "windows")]
    return Ok(base.join("npm.cmd"));

    #[cfg(not(target_os = "windows"))]
    return Ok(base.join("bin").join("npm"));
  }

  /// Get gemini executable path
  pub fn get_exe_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
      .path()
      .app_local_data_dir()
      .map_err(|e| e.to_string())?
      .join("bin")
      .join("nodejs");

    #[cfg(target_os = "windows")]
    return Ok(base.join("gemini.cmd"));

    #[cfg(not(target_os = "windows"))]
    return Ok(base.join("bin").join("gemini"));
  }

  /// Check if Gemini CLI is installed
  pub fn is_installed(app: &AppHandle) -> Result<bool, String> {
    Ok(Self::get_exe_path(app)?.exists())
  }

  /// Get Gemini CLI version
  pub fn get_version(app: &AppHandle) -> Result<Option<String>, String> {
    if !Self::is_installed(app)? {
      return Ok(None);
    }

    let exe = Self::get_exe_path(app)?;
    let output = hidden_command(&exe)
      .args(["--version"])
      .output()
      .ok();

    if let Some(output) = output {
      if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let version = stdout.trim().to_string();
        if !version.is_empty() {
          return Ok(Some(version));
        }
      }
    }

    Ok(None)
  }

  /// Get full Gemini CLI status
  pub fn get_status(app: &AppHandle) -> Result<AiToolStatus, String> {
    let installed = Self::is_installed(app)?;
    let path = if installed {
      Some(Self::get_exe_path(app)?.to_string_lossy().to_string())
    } else {
      None
    };
    let version = if installed {
      Self::get_version(app)?
    } else {
      None
    };

    Ok(AiToolStatus {
      installed,
      path,
      version,
    })
  }

  /// Install Gemini CLI via npm
  pub fn install(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed. Please install Node.js first.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["install", "-g", "@google/gemini-cli"])
      .output()
      .map_err(|e| format!("Failed to run npm install: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }

  /// Uninstall Gemini CLI via npm
  pub fn uninstall(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["uninstall", "-g", "@google/gemini-cli"])
      .output()
      .map_err(|e| format!("Failed to run npm uninstall: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }

  /// Update Gemini CLI via npm
  pub fn update(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["update", "-g", "@google/gemini-cli"])
      .output()
      .map_err(|e| format!("Failed to run npm update: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }
}
