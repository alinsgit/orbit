use crate::services::config::ConfigManager;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

// Service types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceType {
    Nginx,
    Php(u32), // PHP Version (e.g., 82 for 8.2)
    MariaDB,
    Apache,
    NodeJs,
    Python,
    Bun,
}

fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

// Global state to hold running processes
pub struct ServiceManager {
    processes: Arc<Mutex<HashMap<String, Child>>>,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn ensure_config(service_type: ServiceType, bin_path_buf: &PathBuf) {
        if let Some(parent) = bin_path_buf.parent() {
            let root = parent.to_path_buf();
            match service_type {
                ServiceType::Nginx => {
                    let _ = ConfigManager::ensure_nginx_config(&root);
                }
                ServiceType::Php(_) => {
                    let _ = ConfigManager::ensure_php_config(&root);
                }
                ServiceType::MariaDB => {
                    // MariaDB config is handled by start_service with correct app data path
                    // Don't create config here as we don't have the correct data directory
                }
                ServiceType::Apache => {
                    let _ = ConfigManager::ensure_apache_config(&root);
                }
                _ => {}
            }
        }
    }

    #[allow(dead_code)]
    pub fn start(
        &self,
        service_type: ServiceType,
        bin_path: &str,
        args: &[&str],
    ) -> Result<u32, String> {
        let name = format!("{:?}", service_type);
        self.start_with_name(name, service_type, bin_path, args)
    }

    pub fn start_with_name(
        &self,
        name: String,
        service_type: ServiceType,
        bin_path: &str,
        args: &[&str],
    ) -> Result<u32, String> {
        let bin_path_buf = PathBuf::from(bin_path);
        Self::ensure_config(service_type, &bin_path_buf);

        let mut command = Command::new(bin_path);
        command.args(args);

        if let Some(parent) = bin_path_buf.parent() {
            command.current_dir(parent);
        }

        // Set environment variables for PHP FastCGI
        if matches!(service_type, ServiceType::Php(_)) {
            #[cfg(not(target_os = "windows"))]
            {
                command.env("PHP_FCGI_CHILDREN", "4");
            }
            command.env("PHP_FCGI_MAX_REQUESTS", "1000");
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        match command.spawn() {
            Ok(mut child) => {
                let pid = child.id();

                // Wait briefly and verify the process didn't crash immediately
                std::thread::sleep(std::time::Duration::from_millis(1500));

                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        // Process already exited - it crashed
                        Err(format!(
                            "Service {} exited immediately (exit code: {})",
                            name, exit_status
                        ))
                    }
                    Ok(None) => {
                        // Still running - success
                        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;
                        processes.insert(name, child);
                        Ok(pid)
                    }
                    Err(e) => {
                        Err(format!("Failed to check service status: {}", e))
                    }
                }
            }
            Err(e) => Err(format!("Failed to start service: {}", e)),
        }
    }

    pub fn stop(&self, service_name: &str) -> Result<(), String> {
        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;

        if let Some(mut child) = processes.remove(service_name) {
            let pid = child.id();

            // Try taskkill first for process tree cleanup, then fall back to child.kill()
            #[cfg(target_os = "windows")]
            {
                let _ = hidden_command("taskkill")
                    .args(&["/F", "/PID", &pid.to_string(), "/T"])
                    .output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = Command::new("kill").arg(pid.to_string()).output();
            }

            // Ensure child is reaped
            let _ = child.wait();
            Ok(())
        } else {
            Err("Service not found or not running".to_string())
        }
    }

    pub fn get_status(&self, service_name: &str) -> Option<String> {
        let mut processes = self.processes.lock().ok()?;

        let is_alive = if let Some(child) = processes.get_mut(service_name) {
            match child.try_wait() {
                Ok(Some(_)) => false, // Process exited
                Ok(None) => true,     // Still running
                Err(_) => false,      // Error checking, assume dead
            }
        } else {
            return Some("stopped".to_string());
        };

        if !is_alive {
            processes.remove(service_name);
            Some("stopped".to_string())
        } else {
            Some("running".to_string())
        }
    }

    #[allow(dead_code)]
    pub fn start_all(&self) -> Result<Vec<String>, String> {
        let results = Vec::new();
        Ok(results)
    }

    pub fn stop_all(&self) -> Result<(), String> {
        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;
        let service_names: Vec<String> = processes.keys().cloned().collect();

        for name in service_names {
            if let Some(mut child) = processes.remove(&name) {
                let pid = child.id();

                #[cfg(target_os = "windows")]
                {
                    let _ = hidden_command("taskkill")
                        .args(&["/F", "/PID", &pid.to_string(), "/T"])
                        .output();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = Command::new("kill").arg(pid.to_string()).output();
                }

                let _ = child.wait();
            }
        }
        Ok(())
    }

    /// Calculate PHP port based on version (e.g., PHP 8.2 = 9082, PHP 8.3 = 9083)
    fn get_php_port(version: u32) -> u16 {
        9000 + version as u16
    }

    pub fn start_auto(&self, name: String) -> Result<u32, String> {
        let service_type = if name.contains("nginx") {
            ServiceType::Nginx
        } else if name.contains("php") {
            let version: u32 = name
                .split('-')
                .nth(1)
                .map(|v| v.replace(".", ""))
                .and_then(|v| v.parse().ok())
                .unwrap_or(82);
            ServiceType::Php(version)
        } else {
            ServiceType::MariaDB
        };

        let _args: Vec<String> = match service_type {
            ServiceType::Nginx => vec![],
            ServiceType::Php(version) => {
                let port = Self::get_php_port(version);
                vec!["-b".to_string(), format!("127.0.0.1:{}", port)]
            }
            ServiceType::MariaDB => vec![],
            ServiceType::Apache => vec![],
            ServiceType::NodeJs | ServiceType::Python | ServiceType::Bun => vec![],
        };

        Err("Service path not found".to_string())
    }
}
