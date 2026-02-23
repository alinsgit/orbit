use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::hidden_command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStatus {
    pub installed: bool,
    pub running: bool,
    pub path: Option<String>,
    pub pid: Option<u32>,
    pub binary_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryUpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
}

pub struct McpManager;

impl McpManager {
    /// Get MCP directory
    fn get_mcp_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let bin_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("mcp");
        Ok(bin_dir)
    }

    /// Get MCP executable path
    pub fn get_exe_path(app: &AppHandle) -> Result<PathBuf, String> {
        let mcp_dir = Self::get_mcp_dir(app)?;
        #[cfg(target_os = "windows")]
        let exe_name = "orbit-mcp.exe";
        #[cfg(not(target_os = "windows"))]
        let exe_name = "orbit-mcp";
        Ok(mcp_dir.join(exe_name))
    }

    /// Check if MCP is installed
    pub fn is_installed(app: &AppHandle) -> Result<bool, String> {
        let exe_path = Self::get_exe_path(app)?;
        Ok(exe_path.exists())
    }

    /// Check if MCP server is running by looking for the process
    pub fn is_running() -> bool {
        #[cfg(target_os = "windows")]
        {
            let output = hidden_command("tasklist")
                .args(["/FI", "IMAGENAME eq orbit-mcp.exe", "/FO", "CSV", "/NH"])
                .output();
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    stdout.contains("orbit-mcp.exe")
                }
                Err(_) => false,
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let output = hidden_command("pgrep")
                .args(["-f", "orbit-mcp"])
                .output();
            match output {
                Ok(out) => out.status.success(),
                Err(_) => false,
            }
        }
    }

    /// Get PID of running MCP process
    fn get_pid() -> Option<u32> {
        #[cfg(target_os = "windows")]
        {
            let output = hidden_command("tasklist")
                .args(["/FI", "IMAGENAME eq orbit-mcp.exe", "/FO", "CSV", "/NH"])
                .output()
                .ok()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            // CSV format: "orbit-mcp.exe","1234","Console","1","12,345 K"
            for line in stdout.lines() {
                if line.contains("orbit-mcp.exe") {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 2 {
                        let pid_str = parts[1].trim_matches('"');
                        return pid_str.parse().ok();
                    }
                }
            }
            None
        }
        #[cfg(not(target_os = "windows"))]
        {
            let output = hidden_command("pgrep")
                .args(["-f", "orbit-mcp"])
                .output()
                .ok()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().next()?.trim().parse().ok()
        }
    }

    /// Get full MCP status
    pub fn get_status(app: &AppHandle) -> Result<McpStatus, String> {
        let installed = Self::is_installed(app)?;
        let exe_path = Self::get_exe_path(app)?;
        let running = Self::is_running();
        let pid = if running { Self::get_pid() } else { None };

        Ok(McpStatus {
            installed,
            running,
            path: if installed {
                Some(exe_path.to_string_lossy().to_string())
            } else {
                None
            },
            pid,
            binary_exists: exe_path.exists(),
        })
    }

    /// Download and install MCP binary from GitHub releases
    pub async fn install(app: &AppHandle) -> Result<(), String> {
        let mcp_dir = Self::get_mcp_dir(app)?;

        // Create directory
        fs::create_dir_all(&mcp_dir)
            .map_err(|e| format!("Failed to create MCP directory: {}", e))?;

        let url = Self::get_download_url().await;
        let exe_path = Self::get_exe_path(app)?;

        crate::services::download::download_file(&url, &exe_path).await?;

        // Add MCP directory to user PATH so AI tools can use just "orbit-mcp"
        crate::commands::path::add_service_to_path(app.clone(), "mcp".to_string()).ok();

        Ok(())
    }

    /// Uninstall MCP
    pub fn uninstall(app: &AppHandle) -> Result<(), String> {
        let mcp_dir = Self::get_mcp_dir(app)?;

        if mcp_dir.exists() {
            fs::remove_dir_all(&mcp_dir)
                .map_err(|e| format!("Failed to remove MCP: {}", e))?;
        }

        // Clean up PATH entry
        crate::commands::path::remove_service_from_path(app.clone(), "mcp".to_string()).ok();

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
        return "https://github.com/alinsgit/orbit/releases/latest/download/orbit-mcp-windows-x64.exe".to_string();
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "https://github.com/alinsgit/orbit/releases/latest/download/orbit-mcp-macos-arm64".to_string();
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "https://github.com/alinsgit/orbit/releases/latest/download/orbit-mcp-macos-x64".to_string();
        #[cfg(target_os = "linux")]
        return "https://github.com/alinsgit/orbit/releases/latest/download/orbit-mcp-linux-x64".to_string();
    }

    /// Try to fetch the latest release download URL from GitHub API
    async fn fetch_latest_release_url() -> Option<String> {
        #[cfg(target_os = "windows")]
        let asset_name = "orbit-mcp-windows-x64.exe";
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let asset_name = "orbit-mcp-macos-arm64";
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let asset_name = "orbit-mcp-macos-x64";
        #[cfg(target_os = "linux")]
        let asset_name = "orbit-mcp-linux-x64";

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

    /// Start MCP server
    pub fn start(app: &AppHandle) -> Result<(), String> {
        let exe_path = Self::get_exe_path(app)?;

        if !exe_path.exists() {
            return Err("MCP server is not installed".to_string());
        }

        // Check if already running
        if Self::is_running() {
            return Ok(());
        }

        // Redirect stdout/stderr to log file
        let log_path = Self::get_mcp_dir(app)?.join("mcp.log");
        let log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Failed to open log file: {}", e))?;
        let log_err = log_file
            .try_clone()
            .map_err(|e| format!("Failed to clone log handle: {}", e))?;

        hidden_command(&exe_path)
            .arg("--standby")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(log_file))
            .stderr(std::process::Stdio::from(log_err))
            .spawn()
            .map_err(|e| format!("Failed to start MCP server: {}", e))?;

        // Wait for process to appear (up to 3 seconds)
        for _ in 0..6 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if Self::is_running() {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Stop MCP server
    #[cfg(target_os = "windows")]
    pub fn stop() -> Result<(), String> {
        hidden_command("taskkill")
            .args(["/F", "/IM", "orbit-mcp.exe"])
            .output()
            .map_err(|e| format!("Failed to stop MCP server: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn stop() -> Result<(), String> {
        hidden_command("pkill")
            .args(["-f", "orbit-mcp"])
            .output()
            .map_err(|e| format!("Failed to stop MCP server: {}", e))?;
        Ok(())
    }

    /// Get installed MCP version by running orbit-mcp --version
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
            let ver = stdout.trim().to_string();
            if ver.is_empty() { None } else { Some(ver) }
        } else {
            None
        }
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

    /// Update MCP binary: stop → uninstall → install
    pub async fn update(app: &AppHandle) -> Result<(), String> {
        // Stop if running
        Self::stop().ok();
        std::thread::sleep(std::time::Duration::from_millis(500));
        // Uninstall old binary
        Self::uninstall(app)?;
        // Install new binary
        Self::install(app).await?;
        Ok(())
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
