use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::hidden_command;

/// Meilisearch status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeilisearchStatus {
    pub installed: bool,
    pub running: bool,
    pub path: Option<String>,
    pub http_port: u16,
}

pub struct MeilisearchManager;

impl MeilisearchManager {
    /// Get Meilisearch directory
    fn get_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let bin_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("meilisearch");
        Ok(bin_dir)
    }

    /// Get Meilisearch executable path
    pub fn get_exe_path(app: &AppHandle) -> Result<PathBuf, String> {
        let dir = Self::get_dir(app)?;
        #[cfg(target_os = "windows")]
        let exe_name = "meilisearch.exe";
        #[cfg(not(target_os = "windows"))]
        let exe_name = "meilisearch";
        Ok(dir.join(exe_name))
    }

    /// Check if Meilisearch is installed
    pub fn is_installed(app: &AppHandle) -> Result<bool, String> {
        let exe_path = Self::get_exe_path(app)?;
        Ok(exe_path.exists())
    }

    /// Check if Meilisearch is running (port 7700)
    pub fn is_running() -> bool {
        use std::net::TcpListener;
        TcpListener::bind("127.0.0.1:7700").is_err()
    }

    /// Get full Meilisearch status
    pub fn get_status(app: &AppHandle) -> Result<MeilisearchStatus, String> {
        let installed = Self::is_installed(app)?;
        let path = if installed {
            Some(Self::get_exe_path(app)?.to_string_lossy().to_string())
        } else {
            None
        };

        Ok(MeilisearchStatus {
            installed,
            running: Self::is_running(),
            path,
            http_port: 7700,
        })
    }

    /// Download and install Meilisearch
    pub async fn install(app: &AppHandle) -> Result<(), String> {
        let dir = Self::get_dir(app)?;

        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create Meilisearch directory: {e}"))?;

        let url = Self::get_download_url().await;
        let exe_path = Self::get_exe_path(app)?;

        // Meilisearch is a single exe download
        crate::services::download::download_file(&url, &exe_path).await?;

        Ok(())
    }

    /// Uninstall Meilisearch
    pub fn uninstall(app: &AppHandle) -> Result<(), String> {
        let dir = Self::get_dir(app)?;

        if dir.exists() {
            fs::remove_dir_all(&dir)
                .map_err(|e| format!("Failed to remove Meilisearch: {e}"))?;
        }

        Ok(())
    }

    /// Get download URL for current platform from registry with fallback
    async fn get_download_url() -> String {
        if let Ok(registry) = crate::services::registry::LibraryRegistry::get().await {
            if let Some(url) = registry.get_download_url("meilisearch", None) {
                return url;
            }
        }
        #[cfg(target_os = "windows")]
        return "https://github.com/meilisearch/meilisearch/releases/download/v1.36.0/meilisearch-windows-amd64.exe".to_string();
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "https://github.com/meilisearch/meilisearch/releases/download/v1.36.0/meilisearch-macos-apple-silicon".to_string();
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "https://github.com/meilisearch/meilisearch/releases/download/v1.36.0/meilisearch-macos-amd64".to_string();
        #[cfg(target_os = "linux")]
        return "https://github.com/meilisearch/meilisearch/releases/download/v1.36.0/meilisearch-linux-amd64".to_string();
    }

    /// Start Meilisearch server
    pub fn start(app: &AppHandle) -> Result<(), String> {
        let exe_path = Self::get_exe_path(app)?;

        if !exe_path.exists() {
            return Err("Meilisearch is not installed".to_string());
        }

        if Self::is_running() {
            return Ok(());
        }

        let dir = Self::get_dir(app)?;
        let db_path = dir.join("data.ms");
        let dump_dir = dir.join("dumps");

        // Redirect stdout/stderr to log file
        let log_path = dir.join("meilisearch.log");
        let log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Failed to open log file: {e}"))?;
        let log_err = log_file
            .try_clone()
            .map_err(|e| format!("Failed to clone log handle: {e}"))?;

        hidden_command(&exe_path)
            .args([
                "--http-addr",
                "127.0.0.1:7700",
                "--db-path",
                &db_path.to_string_lossy(),
                "--dump-dir",
                &dump_dir.to_string_lossy(),
                "--no-analytics",
            ])
            .stdout(std::process::Stdio::from(log_file))
            .stderr(std::process::Stdio::from(log_err))
            .spawn()
            .map_err(|e| format!("Failed to start Meilisearch: {e}"))?;

        // Wait for port to become available (up to 5 seconds)
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if Self::is_running() {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Stop Meilisearch server
    #[cfg(target_os = "windows")]
    pub fn stop() -> Result<(), String> {
        hidden_command("taskkill")
            .args(["/F", "/IM", "meilisearch.exe"])
            .output()
            .map_err(|e| format!("Failed to stop Meilisearch: {e}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn stop() -> Result<(), String> {
        hidden_command("pkill")
            .args(["-f", "meilisearch"])
            .output()
            .map_err(|e| format!("Failed to stop Meilisearch: {}", e))?;
        Ok(())
    }
}
