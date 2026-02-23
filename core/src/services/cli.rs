use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::hidden_command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub binary_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryUpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
}

pub struct CliManager;

impl CliManager {
    /// Get CLI directory
    fn get_cli_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let bin_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("cli");
        Ok(bin_dir)
    }

    /// Get CLI executable path
    pub fn get_exe_path(app: &AppHandle) -> Result<PathBuf, String> {
        let cli_dir = Self::get_cli_dir(app)?;
        #[cfg(target_os = "windows")]
        let exe_name = "orbit-cli.exe";
        #[cfg(not(target_os = "windows"))]
        let exe_name = "orbit-cli";
        Ok(cli_dir.join(exe_name))
    }

    /// Check if CLI is installed (in any known location)
    pub fn is_installed(app: &AppHandle) -> Result<bool, String> {
        let exe_path = Self::get_exe_path(app)?;
        Ok(exe_path.exists())
    }

    /// Get CLI version by running orbit-cli --version
    fn get_version(app: &AppHandle) -> Option<String> {
        let exe_path = Self::get_exe_path(app).ok()?;
        if !exe_path.exists() {
            return None;
        }

        let output = hidden_command(&exe_path)
            .arg("--version")
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let version = stdout.trim().to_string();
            if version.is_empty() {
                None
            } else {
                Some(version)
            }
        } else {
            None
        }
    }

    /// Get full CLI status
    pub fn get_status(app: &AppHandle) -> Result<CliStatus, String> {
        let installed = Self::is_installed(app)?;
        let exe_path = Self::get_exe_path(app)?;
        let version = if installed {
            Self::get_version(app)
        } else {
            None
        };

        Ok(CliStatus {
            installed,
            path: if installed {
                Some(exe_path.to_string_lossy().to_string())
            } else {
                None
            },
            version,
            binary_exists: exe_path.exists(),
        })
    }

    /// Download and install CLI binary from GitHub releases
    pub async fn install(app: &AppHandle) -> Result<(), String> {
        let cli_dir = Self::get_cli_dir(app)?;

        // Create directory
        fs::create_dir_all(&cli_dir)
            .map_err(|e| format!("Failed to create CLI directory: {}", e))?;

        let url = Self::get_download_url().await;
        let exe_path = Self::get_exe_path(app)?;

        crate::services::download::download_file(&url, &exe_path).await?;

        // Add CLI directory to user PATH so terminal can use just "orbit-cli"
        crate::commands::path::add_service_to_path(app.clone(), "cli".to_string()).ok();

        Ok(())
    }

    /// Uninstall CLI
    pub fn uninstall(app: &AppHandle) -> Result<(), String> {
        let cli_dir = Self::get_cli_dir(app)?;

        if cli_dir.exists() {
            fs::remove_dir_all(&cli_dir)
                .map_err(|e| format!("Failed to remove CLI: {}", e))?;
        }

        // Clean up PATH entry
        crate::commands::path::remove_service_from_path(app.clone(), "cli".to_string()).ok();

        Ok(())
    }

    /// Get download URL for current platform from GitHub releases
    async fn get_download_url() -> String {
        // Try to get latest release URL from GitHub API
        if let Some(url) = Self::fetch_latest_release_url().await {
            return url;
        }
        // Fallback to a known version
        #[cfg(target_os = "windows")]
        return "https://github.com/alinsgit/orbit/releases/latest/download/orbit-cli-windows-x64.exe".to_string();
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "https://github.com/alinsgit/orbit/releases/latest/download/orbit-cli-macos-arm64".to_string();
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "https://github.com/alinsgit/orbit/releases/latest/download/orbit-cli-macos-x64".to_string();
        #[cfg(target_os = "linux")]
        return "https://github.com/alinsgit/orbit/releases/latest/download/orbit-cli-linux-x64".to_string();
    }

    /// Fetch latest release tag from GitHub
    async fn fetch_latest_version() -> Option<String> {
        let client = reqwest::Client::builder()
            .user_agent("Orbit")
            .build()
            .ok()?;

        let response = client
            .get("https://api.github.com/repos/alinsgit/orbit/releases/latest")
            .send()
            .await
            .ok()?;

        let release: serde_json::Value = response.json().await.ok()?;
        let tag = release.get("tag_name")?.as_str()?;
        Some(tag.trim_start_matches('v').to_string())
    }

    /// Check if an update is available
    pub async fn check_for_update(app: &AppHandle) -> Result<BinaryUpdateInfo, String> {
        let current = Self::get_version(app).unwrap_or_else(|| "0.0.0".to_string());
        let latest = Self::fetch_latest_version().await
            .ok_or_else(|| "Failed to fetch latest version from GitHub".to_string())?;

        let has_update = is_newer_version(&current, &latest);

        Ok(BinaryUpdateInfo {
            has_update,
            current_version: current,
            latest_version: latest,
        })
    }

    /// Update CLI binary: uninstall → install
    pub async fn update(app: &AppHandle) -> Result<(), String> {
        Self::uninstall(app)?;
        Self::install(app).await?;
        Ok(())
    }

    /// Try to fetch the latest release download URL from GitHub API
    async fn fetch_latest_release_url() -> Option<String> {
        #[cfg(target_os = "windows")]
        let asset_name = "orbit-cli-windows-x64.exe";
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let asset_name = "orbit-cli-macos-arm64";
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let asset_name = "orbit-cli-macos-x64";
        #[cfg(target_os = "linux")]
        let asset_name = "orbit-cli-linux-x64";

        let client = reqwest::Client::builder()
            .user_agent("Orbit")
            .build()
            .ok()?;

        let response = client
            .get("https://api.github.com/repos/alinsgit/orbit/releases/latest")
            .send()
            .await
            .ok()?;

        let release: serde_json::Value = response.json().await.ok()?;
        let assets = release.get("assets")?.as_array()?;

        for asset in assets {
            let name = asset.get("name")?.as_str()?;
            if name == asset_name {
                return asset
                    .get("browser_download_url")?
                    .as_str()
                    .map(|s| s.to_string());
            }
        }

        None
    }
}

/// Compare two semver version strings. Returns true if `latest` is newer than `current`.
fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    let current_parts = parse(current);
    let latest_parts = parse(latest);
    for i in 0..3 {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let l = latest_parts.get(i).copied().unwrap_or(0);
        if l > c { return true; }
        if l < c { return false; }
    }
    false
}
