use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::hidden_command;

/// Mailpit status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailpitStatus {
    pub installed: bool,
    pub running: bool,
    pub path: Option<String>,
    pub smtp_port: u16,
    pub web_port: u16,
}

pub struct MailpitManager;

impl MailpitManager {
    /// Get Mailpit directory
    fn get_mailpit_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let bin_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("mailpit");
        Ok(bin_dir)
    }

    /// Get Mailpit executable path
    pub fn get_exe_path(app: &AppHandle) -> Result<PathBuf, String> {
        let mailpit_dir = Self::get_mailpit_dir(app)?;
        #[cfg(target_os = "windows")]
        let exe_name = "mailpit.exe";
        #[cfg(not(target_os = "windows"))]
        let exe_name = "mailpit";
        Ok(mailpit_dir.join(exe_name))
    }

    /// Check if Mailpit is installed
    pub fn is_installed(app: &AppHandle) -> Result<bool, String> {
        let exe_path = Self::get_exe_path(app)?;
        Ok(exe_path.exists())
    }

    /// Check if Mailpit is running
    pub fn is_running() -> bool {
        Self::check_port_in_use(8025) || Self::check_port_in_use(1025)
    }

    /// Check if a port is in use
    fn check_port_in_use(port: u16) -> bool {
        use std::net::TcpListener;
        TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
    }

    /// Get full Mailpit status
    pub fn get_status(app: &AppHandle) -> Result<MailpitStatus, String> {
        let installed = Self::is_installed(app)?;
        let path = if installed {
            Some(Self::get_exe_path(app)?.to_string_lossy().to_string())
        } else {
            None
        };

        Ok(MailpitStatus {
            installed,
            running: Self::is_running(),
            path,
            smtp_port: 1025,
            web_port: 8025,
        })
    }

    /// Download and install Mailpit
    pub async fn install(app: &AppHandle) -> Result<(), String> {
        let mailpit_dir = Self::get_mailpit_dir(app)?;

        // Create directory
        fs::create_dir_all(&mailpit_dir)
            .map_err(|e| format!("Failed to create Mailpit directory: {}", e))?;

        let url = Self::get_download_url().await;

        if url.ends_with(".zip") {
            // Download zip to temp location, then extract
            let zip_path = mailpit_dir.join("mailpit-download.zip");
            crate::services::download::download_file(&url, &zip_path).await?;
            crate::services::download::extract_zip(&zip_path, &mailpit_dir)?;
            // Clean up zip
            fs::remove_file(&zip_path).ok();
        } else {
            // Direct exe download (legacy)
            let exe_path = Self::get_exe_path(app)?;
            crate::services::download::download_file(&url, &exe_path).await?;
        }

        Ok(())
    }

    /// Uninstall Mailpit
    pub fn uninstall(app: &AppHandle) -> Result<(), String> {
        let mailpit_dir = Self::get_mailpit_dir(app)?;

        if mailpit_dir.exists() {
            fs::remove_dir_all(&mailpit_dir)
                .map_err(|e| format!("Failed to remove Mailpit: {}", e))?;
        }

        Ok(())
    }
    /// Get download URL for current platform from registry with fallback
    async fn get_download_url() -> String {
        // Try to get from registry
        if let Ok(registry) = crate::services::registry::LibraryRegistry::get().await {
            if let Some(url) = registry.get_download_url("mailpit", None) {
                return url;
            }
        }
        // Platform-specific fallback
        #[cfg(target_os = "windows")]
        return "https://github.com/axllent/mailpit/releases/download/v1.29.0/mailpit-windows-amd64.zip".to_string();
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "https://github.com/axllent/mailpit/releases/download/v1.29.0/mailpit-darwin-arm64.tar.gz".to_string();
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "https://github.com/axllent/mailpit/releases/download/v1.29.0/mailpit-darwin-amd64.tar.gz".to_string();
        #[cfg(target_os = "linux")]
        return "https://github.com/axllent/mailpit/releases/download/v1.29.0/mailpit-linux-amd64.tar.gz".to_string();
    }

    /// Start Mailpit server
    pub fn start(app: &AppHandle) -> Result<(), String> {
        let exe_path = Self::get_exe_path(app)?;

        if !exe_path.exists() {
            return Err("Mailpit is not installed".to_string());
        }

        // Check if already running
        if Self::is_running() {
            return Ok(());
        }

        // Redirect stdout/stderr to log file
        let log_path = Self::get_mailpit_dir(app)?.join("mailpit.log");
        let log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Failed to open log file: {}", e))?;
        let log_err = log_file.try_clone()
            .map_err(|e| format!("Failed to clone log handle: {}", e))?;

        hidden_command(&exe_path)
            .args([
                "--smtp", "127.0.0.1:1025",
                "--listen", "127.0.0.1:8025",
            ])
            .stdout(std::process::Stdio::from(log_file))
            .stderr(std::process::Stdio::from(log_err))
            .spawn()
            .map_err(|e| format!("Failed to start Mailpit: {}", e))?;

        // Wait for port to become available (up to 3 seconds)
        for _ in 0..6 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if Self::is_running() {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Stop Mailpit server
    #[cfg(target_os = "windows")]
    pub fn stop() -> Result<(), String> {
        hidden_command("taskkill")
            .args(["/F", "/IM", "mailpit.exe"])
            .output()
            .map_err(|e| format!("Failed to stop Mailpit: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn stop() -> Result<(), String> {
        hidden_command("pkill")
            .args(["-f", "mailpit"])
            .output()
            .map_err(|e| format!("Failed to stop Mailpit: {}", e))?;
        Ok(())
    }
}
