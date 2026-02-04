use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// PHP Service entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpService {
    pub version: String,       // e.g., "8.4", "8.5"
    pub port: u16,             // e.g., 9004, 9005
    pub path: String,          // Path to PHP installation
    pub status: PhpStatus,     // running, stopped
    pub pid: Option<u32>,      // Process ID if running
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PhpStatus {
    Running,
    Stopped,
}

/// PHP Services Registry
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhpRegistry {
    pub version: String,
    pub services: Vec<PhpService>,
}

impl PhpRegistry {
    const FILENAME: &'static str = "php_services.json";

    /// Get registry file path
    fn get_registry_path(app: &AppHandle) -> Result<PathBuf, String> {
        let config_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("config");

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        Ok(config_dir.join(Self::FILENAME))
    }

    /// Load registry from file
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let path = Self::get_registry_path(app)?;

        if !path.exists() {
            return Ok(Self::default_registry());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read PHP registry: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse PHP registry: {}", e))
    }

    /// Save registry to file
    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let path = Self::get_registry_path(app)?;

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize PHP registry: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Failed to write PHP registry: {}", e))
    }

    /// Create default registry with common PHP versions
    fn default_registry() -> Self {
        Self {
            version: "1.0".to_string(),
            services: vec![],
        }
    }

    /// Calculate port from PHP version
    /// PHP 8.4 -> 9004, PHP 8.5 -> 9005, PHP 7.4 -> 9074
    pub fn calculate_port(version: &str) -> u16 {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                if major == 8 {
                    return 9000 + minor;
                }
                // For PHP 7.x: 9070 + minor (7.4 -> 9074)
                return 9000 + (major % 10) * 10 + minor;
            }
        }
        9000 // Default fallback
    }

    /// Register a new PHP version
    pub fn register_php(&mut self, version: &str, path: &str) -> &PhpService {
        // Check if already registered
        if let Some(idx) = self.services.iter().position(|s| s.version == version) {
            // Update path if different
            self.services[idx].path = path.to_string();
            return &self.services[idx];
        }

        // Calculate port
        let port = Self::calculate_port(version);

        // Create new service entry
        let service = PhpService {
            version: version.to_string(),
            port,
            path: path.to_string(),
            status: PhpStatus::Stopped,
            pid: None,
        };

        self.services.push(service);
        self.services.last().unwrap()
    }

    /// Unregister a PHP version
    pub fn unregister_php(&mut self, version: &str) -> bool {
        if let Some(idx) = self.services.iter().position(|s| s.version == version) {
            self.services.remove(idx);
            true
        } else {
            false
        }
    }

    /// Get PHP service by version
    pub fn get_service(&self, version: &str) -> Option<&PhpService> {
        self.services.iter().find(|s| s.version == version)
    }

    /// Get PHP service by version (mutable)
    pub fn get_service_mut(&mut self, version: &str) -> Option<&mut PhpService> {
        self.services.iter_mut().find(|s| s.version == version)
    }

    /// Get port for a PHP version
    pub fn get_port(&self, version: &str) -> Option<u16> {
        self.get_service(version).map(|s| s.port)
    }

    /// Get port or calculate if not registered
    pub fn get_or_calculate_port(&self, version: &str) -> u16 {
        self.get_port(version).unwrap_or_else(|| Self::calculate_port(version))
    }

    /// Update service status
    pub fn set_status(&mut self, version: &str, status: PhpStatus, pid: Option<u32>) -> bool {
        if let Some(service) = self.get_service_mut(version) {
            service.status = status;
            service.pid = pid;
            true
        } else {
            false
        }
    }

    /// Mark service as running
    pub fn mark_running(&mut self, version: &str, pid: u32) -> bool {
        self.set_status(version, PhpStatus::Running, Some(pid))
    }

    /// Mark service as stopped
    pub fn mark_stopped(&mut self, version: &str) -> bool {
        self.set_status(version, PhpStatus::Stopped, None)
    }

    /// Get all running services
    pub fn get_running_services(&self) -> Vec<&PhpService> {
        self.services
            .iter()
            .filter(|s| s.status == PhpStatus::Running)
            .collect()
    }

    /// Check if a port is already in use by another PHP version
    #[allow(dead_code)]
    pub fn is_port_in_use(&self, port: u16, exclude_version: Option<&str>) -> bool {
        self.services.iter().any(|s| {
            s.port == port && exclude_version.map_or(true, |v| s.version != v)
        })
    }

    /// Scan installed PHP versions and update registry
    pub fn scan_installed_versions(&mut self, app: &AppHandle) -> Result<usize, String> {
        let php_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("php");

        if !php_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;

        if let Ok(entries) = fs::read_dir(&php_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(version) = path.file_name().and_then(|n| n.to_str()) {
                        // Check if php-cgi.exe exists
                        let php_cgi = path.join("php-cgi.exe");
                        if php_cgi.exists() {
                            self.register_php(version, path.to_string_lossy().as_ref());
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Verify running services (check if PIDs are still alive)
    pub fn verify_running_services(&mut self) {
        for service in &mut self.services {
            if service.status == PhpStatus::Running {
                if let Some(pid) = service.pid {
                    // Check if process is still running
                    if !Self::is_process_running(pid) {
                        service.status = PhpStatus::Stopped;
                        service.pid = None;
                    }
                } else {
                    // No PID but marked as running - mark as stopped
                    service.status = PhpStatus::Stopped;
                }
            }
        }
    }

    /// Check if a process is running (Windows)
    #[cfg(windows)]
    fn is_process_running(pid: u32) -> bool {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }

    #[cfg(not(windows))]
    fn is_process_running(pid: u32) -> bool {
        use std::path::Path;
        Path::new(&format!("/proc/{}", pid)).exists()
    }
}

/// Helper functions for use in other modules
#[allow(dead_code)]
pub fn get_php_port(app: &AppHandle, version: &str) -> Result<u16, String> {
    let registry = PhpRegistry::load(app)?;
    Ok(registry.get_or_calculate_port(version))
}

#[allow(dead_code)]
pub fn register_and_get_port(app: &AppHandle, version: &str, path: &str) -> Result<u16, String> {
    let mut registry = PhpRegistry::load(app)?;
    let service = registry.register_php(version, path);
    let port = service.port;
    registry.save(app)?;
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_calculation() {
        assert_eq!(PhpRegistry::calculate_port("8.4"), 9004);
        assert_eq!(PhpRegistry::calculate_port("8.5"), 9005);
        assert_eq!(PhpRegistry::calculate_port("8.3"), 9003);
        assert_eq!(PhpRegistry::calculate_port("7.4"), 9074);
        assert_eq!(PhpRegistry::calculate_port("7.3"), 9073);
    }

    #[test]
    fn test_register_php() {
        let mut registry = PhpRegistry::default_registry();

        registry.register_php("8.4", "/path/to/php84");
        assert_eq!(registry.services.len(), 1);
        assert_eq!(registry.get_port("8.4"), Some(9004));

        registry.register_php("8.5", "/path/to/php85");
        assert_eq!(registry.services.len(), 2);
        assert_eq!(registry.get_port("8.5"), Some(9005));
    }
}
