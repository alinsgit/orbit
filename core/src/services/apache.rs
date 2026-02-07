use std::fs;
use std::path::PathBuf;
#[cfg(not(windows))]
use std::process::Command;
use tauri::{AppHandle, Manager};

use super::hidden_command;

pub struct ApacheManager;

impl ApacheManager {
    /// Get Apache bin directory
    pub fn get_apache_path(app: &AppHandle) -> Result<PathBuf, String> {
        let apache_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("apache");

        Ok(apache_dir)
    }

    /// Get Apache conf directory
    pub fn get_config_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let config_dir = Self::get_apache_path(app)?.join("conf");
        Ok(config_dir)
    }

    /// Get virtual hosts directory
    pub fn get_vhosts_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let vhosts_dir = Self::get_config_dir(app)?.join("vhosts");

        if !vhosts_dir.exists() {
            fs::create_dir_all(&vhosts_dir)
                .map_err(|e| format!("Failed to create vhosts dir: {}", e))?;
        }

        Ok(vhosts_dir)
    }

    /// Check if Apache is installed
    #[allow(dead_code)]
    pub fn is_installed(app: &AppHandle) -> bool {
        if let Ok(apache_path) = Self::get_apache_path(app) {
            apache_path.join("bin").join("httpd.exe").exists()
        } else {
            false
        }
    }

    /// Test Apache configuration
    pub fn test_config(app: &AppHandle) -> Result<String, String> {
        let apache_path = Self::get_apache_path(app)?;
        let httpd = apache_path.join("bin").join("httpd.exe");

        if !httpd.exists() {
            return Err("Apache httpd.exe not found".to_string());
        }

        let output = hidden_command(&httpd)
            .arg("-t")
            .current_dir(&apache_path)
            .output()
            .map_err(|e| format!("Failed to run httpd: {}", e))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() || stderr.contains("Syntax OK") {
            Ok("Syntax OK".to_string())
        } else {
            Err(format!("Config error: {}", stderr))
        }
    }

    /// Reload Apache (graceful restart)
    pub fn reload(app: &AppHandle) -> Result<String, String> {
        let apache_path = Self::get_apache_path(app)?;
        let httpd = apache_path.join("bin").join("httpd.exe");

        if !httpd.exists() {
            return Err("Apache httpd.exe not found".to_string());
        }

        // First test config
        Self::test_config(app)?;

        // Graceful restart
        let output = hidden_command(&httpd)
            .arg("-k")
            .arg("restart")
            .current_dir(&apache_path)
            .output()
            .map_err(|e| format!("Failed to reload Apache: {}", e))?;

        if output.status.success() {
            Ok("Apache reloaded".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Sometimes restart returns non-zero but still works
            if stderr.is_empty() {
                Ok("Apache reload signal sent".to_string())
            } else {
                Err(format!("Reload error: {}", stderr))
            }
        }
    }

    /// Ensure main httpd.conf includes vhosts directory
    pub fn ensure_main_config(app: &AppHandle) -> Result<(), String> {
        let config_dir = Self::get_config_dir(app)?;
        let httpd_conf = config_dir.join("httpd.conf");

        if !httpd_conf.exists() {
            return Err("httpd.conf not found".to_string());
        }

        let content = fs::read_to_string(&httpd_conf)
            .map_err(|e| format!("Failed to read httpd.conf: {}", e))?;

        // Check if vhosts include already exists
        let include_line = "Include conf/vhosts/*.conf";
        if content.contains(include_line) {
            return Ok(());
        }

        // Also check for commented version
        let include_pattern = "conf/vhosts/";
        if content.contains(include_pattern) {
            return Ok(());
        }

        // Append vhosts include to httpd.conf
        let new_content = format!(
            "{}\n\n# Include virtual hosts\n{}\n",
            content.trim_end(),
            include_line
        );

        fs::write(&httpd_conf, new_content)
            .map_err(|e| format!("Failed to write httpd.conf: {}", e))?;

        // Ensure vhosts directory exists
        Self::get_vhosts_dir(app)?;

        Ok(())
    }

    /// Check if Apache is running (check for httpd process)
    #[allow(dead_code)]
    pub fn is_running() -> bool {
        #[cfg(windows)]
        {
            let output = hidden_command("tasklist")
                .args(["/FI", "IMAGENAME eq httpd.exe"])
                .output();

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains("httpd.exe")
            } else {
                false
            }
        }

        #[cfg(not(windows))]
        {
            let output = Command::new("pgrep")
                .arg("-x")
                .arg("httpd")
                .output();

            output.map(|o| o.status.success()).unwrap_or(false)
        }
    }
}
