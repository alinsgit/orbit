use crate::services::config::ConfigManager;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use sysinfo::{SystemExt, ProcessExt};

// Service types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceType {
    Nginx,
    Php(u32), // PHP Version (e.g., 82 for 8.2)
    MariaDB,
    PostgreSQL,
    MongoDB,
    Apache,
    NodeJs,
    Python,
    Bun,
    Redis,
    Go,
    Deno,
    Mailpit,
}

#[cfg(target_os = "windows")]
use super::hidden_command;

/// Check if a port is in use
fn is_port_in_use(port: u16) -> bool {
    use std::net::TcpListener;
    // Check both addresses — on Windows, services may bind to either
    TcpListener::bind(format!("127.0.0.1:{port}")).is_err()
        || TcpListener::bind(format!("0.0.0.0:{port}")).is_err()
}

/// Get expected port for a service by name
fn get_service_port(service_name: &str) -> Option<u16> {
    if service_name.contains("mariadb") || service_name.contains("mysql") {
        Some(3306)
    } else if service_name.contains("postgres") {
        Some(5432)
    } else if service_name.contains("mongo") {
        Some(27017)
    } else if service_name.contains("nginx") || service_name.contains("apache") || service_name.contains("httpd") {
        Some(80)
    } else if service_name.contains("php") {
        if let Some(version_str) = service_name.strip_prefix("php-") {
            let parts: Vec<&str> = version_str.split('.').collect();
            if parts.len() >= 2 {
                let minor: u16 = parts[1].parse().unwrap_or(4);
                Some(9000 + minor)
            } else {
                Some(9004)
            }
        } else {
            Some(9004)
        }
    } else if service_name.contains("redis") {
        Some(6379)
    } else if service_name.contains("mailpit") {
        Some(8025)
    } else if service_name.contains("meilisearch") {
        Some(7700)
    } else {
        None
    }
}

/// Get process image names for taskkill when stopping orphaned processes
fn get_process_names(service_name: &str) -> Vec<&'static str> {
    if service_name.contains("mariadb") || service_name.contains("mysql") {
        vec!["mariadbd.exe", "mysqld.exe"]
    } else if service_name.contains("postgres") {
        vec!["postgres.exe"]
    } else if service_name.contains("mongo") {
        vec!["mongod.exe"]
    } else if service_name.contains("nginx") {
        vec!["nginx.exe"]
    } else if service_name.contains("apache") || service_name.contains("httpd") {
        vec!["httpd.exe"]
    } else if service_name.contains("php") {
        vec!["php-cgi.exe"]
    } else if service_name.contains("redis") {
        vec!["redis-server.exe"]
    } else if service_name.contains("mailpit") {
        vec!["mailpit.exe"]
    } else if service_name.contains("meilisearch") {
        vec!["meilisearch.exe"]
    } else {
        vec![]
    }
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

    fn ensure_config(service_type: ServiceType, bin_path_buf: &Path) {
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
        let name = format!("{service_type:?}");
        self.start_with_name(name, service_type, bin_path, args)
    }

    pub fn start_with_name(
        &self,
        name: String,
        service_type: ServiceType,
        bin_path: &str,
        args: &[&str],
    ) -> Result<u32, String> {
        // Check if service is already running (covers orphaned processes)
        if let Some(port) = get_service_port(&name) {
            if is_port_in_use(port) {
                log::info!("Service {name} already running on port {port}");
                return Ok(0); // Already running, report success
            }
        }

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

                // Runtimes (Bun, NodeJs, Python) are not background daemons.
                // They exit immediately when launched without a script argument.
                // Don't treat immediate exit as a crash for these types.
                let is_runtime = matches!(
                    service_type,
                    ServiceType::Bun | ServiceType::NodeJs | ServiceType::Python | ServiceType::Go | ServiceType::Deno
                );

                if is_runtime {
                    // For runtimes, just register the process without crash-checking
                    let mut processes = self.processes.lock().map_err(|e| e.to_string())?;
                    processes.insert(name, child);
                    Ok(pid)
                } else {
                    // Wait briefly and verify the process didn't crash immediately
                    std::thread::sleep(std::time::Duration::from_millis(1500));

                    match child.try_wait() {
                        Ok(Some(exit_status)) => {
                            // Process already exited - it crashed
                            Err(format!(
                                "Service {name} exited immediately (exit code: {exit_status})"
                            ))
                        }
                        Ok(None) => {
                            // Still running - success
                            let mut processes =
                                self.processes.lock().map_err(|e| e.to_string())?;
                            processes.insert(name, child);
                            Ok(pid)
                        }
                        Err(e) => Err(format!("Failed to check service status: {e}")),
                    }
                }
            }
            Err(e) => Err(format!("Failed to start service: {e}")),
        }
    }

    pub fn stop(&self, service_name: &str) -> Result<(), String> {
        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;

        if let Some(mut child) = processes.remove(service_name) {
            let pid = child.id();

            #[cfg(target_os = "windows")]
            {
                let _ = hidden_command("taskkill")
                    .args(["/F", "/PID", &pid.to_string(), "/T"])
                    .output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = Command::new("kill").arg(pid.to_string()).output();
            }

            let _ = child.wait();
            Ok(())
        } else {
            // Not in HashMap — try to kill orphaned process by image name
            let process_names = get_process_names(service_name);
            let mut killed = false;

            #[cfg(target_os = "windows")]
            for pname in &process_names {
                let output = hidden_command("taskkill")
                    .args(["/F", "/IM", pname, "/T"])
                    .output();
                if let Ok(o) = output {
                    if o.status.success() {
                        killed = true;
                        break;
                    }
                }
            }

            #[cfg(not(target_os = "windows"))]
            for pname in &process_names {
                let _ = Command::new("killall").arg(pname).output();
                killed = true;
            }

            if killed {
                Ok(())
            } else {
                Err("Service not found or not running".to_string())
            }
        }
    }

    pub fn get_status(&self, service_name: &str) -> Option<String> {
        let mut processes = self.processes.lock().ok()?;

        // Check in-memory tracked processes first
        if let Some(child) = processes.get_mut(service_name) {
            match child.try_wait() {
                Ok(Some(_)) => {
                    processes.remove(service_name);
                    // Fall through to port check
                }
                Ok(None) => return Some("running".to_string()),
                Err(_) => {
                    processes.remove(service_name);
                    // Fall through to port check
                }
            }
        }

        // Not tracked or exited — check if running externally (orphan from previous session)
        if let Some(port) = get_service_port(service_name) {
            if is_port_in_use(port) {
                if Self::is_orbit_process_running(service_name) {
                    return Some("running".to_string());
                } else {
                    return Some("stopped".to_string());
                }
            }
        }

        Some("stopped".to_string())
    }

    fn is_orbit_process_running(service_name: &str) -> bool {
        let process_names = get_process_names(service_name);
        if process_names.is_empty() {
            return true;
        }

        let mut sys = sysinfo::System::new();
        sys.refresh_processes();

        let orbit_bin_dir = crate::services::paths::get_bin_dir().to_string_lossy().to_string();
        let clean_dir: String = orbit_bin_dir.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();

        for process in sys.processes().values() {
            let dbg_name = format!("{:?}", process.name());
            let clean_name: String = dbg_name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();

            let matches_name = process_names.iter().any(|&n| {
                let clean_n: String = n.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
                clean_name.contains(&clean_n)
            });

            if matches_name {
                let dbg_exe = format!("{:?}", process.exe());
                let clean_exe: String = dbg_exe.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
                if clean_exe.contains(&clean_dir) {
                    return true;
                }
            }
        }

        false
    }

    pub fn stop_all(&self) -> Result<(), String> {
        // Phase 1: Kill tracked processes from HashMap
        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;
        let service_names: Vec<String> = processes.keys().cloned().collect();

        for name in service_names {
            if let Some(mut child) = processes.remove(&name) {
                let pid = child.id();

                #[cfg(target_os = "windows")]
                {
                    let _ = hidden_command("taskkill")
                        .args(["/F", "/PID", &pid.to_string(), "/T"])
                        .output();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = Command::new("kill").arg(pid.to_string()).output();
                }

                let _ = child.wait();
            }
        }
        drop(processes); // Release lock before phase 2

        // Phase 2: Kill any orphaned Orbit processes by checking known service executables
        // This catches processes from previous sessions or started externally
        let orbit_bin_dir = crate::services::paths::get_bin_dir();
        let clean_bin_dir: String = orbit_bin_dir.to_string_lossy()
            .chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();

        let known_executables: &[&str] = &[
            "nginx.exe", "php-cgi.exe", "mariadbd.exe", "mysqld.exe",
            "postgres.exe", "mongod.exe", "httpd.exe", "redis-server.exe",
            "mailpit.exe",
            "meilisearch.exe",
        ];

        let mut sys = sysinfo::System::new();
        sys.refresh_processes();

        for process in sys.processes().values() {
            let exe_path = format!("{:?}", process.exe());
            let clean_exe: String = exe_path.chars()
                .filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();

            // Only kill processes running from Orbit's bin directory
            if !clean_exe.contains(&clean_bin_dir) {
                continue;
            }

            let proc_name = format!("{:?}", process.name());
            let clean_name: String = proc_name.chars()
                .filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();

            let is_orbit_service = known_executables.iter().any(|&exe| {
                let clean_known: String = exe.chars()
                    .filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
                clean_name.contains(&clean_known)
            });

            if is_orbit_service {
                let pid = process.pid();
                #[cfg(target_os = "windows")]
                {
                    let _ = hidden_command("taskkill")
                        .args(["/F", "/PID", &pid.to_string(), "/T"])
                        .output();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = Command::new("kill").arg(pid.to_string()).output();
                }
            }
        }

        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_service_port_web_servers() {
        assert_eq!(get_service_port("nginx"), Some(80));
        assert_eq!(get_service_port("apache"), Some(80));
        assert_eq!(get_service_port("httpd"), Some(80));
    }

    #[test]
    fn test_get_service_port_databases() {
        assert_eq!(get_service_port("mariadb"), Some(3306));
        assert_eq!(get_service_port("mysql"), Some(3306));
        assert_eq!(get_service_port("postgres"), Some(5432));
        assert_eq!(get_service_port("postgresql"), Some(5432));
        assert_eq!(get_service_port("mongo"), Some(27017));
        assert_eq!(get_service_port("mongodb"), Some(27017));
        assert_eq!(get_service_port("redis"), Some(6379));
    }

    #[test]
    fn test_get_service_port_php() {
        assert_eq!(get_service_port("php-8.4"), Some(9004));
        assert_eq!(get_service_port("php-8.0"), Some(9000));
        assert_eq!(get_service_port("php-7.4"), Some(9004));
        assert_eq!(get_service_port("php-8.5"), Some(9005));
        assert_eq!(get_service_port("php-8.2"), Some(9002));
        assert_eq!(get_service_port("php-8"), Some(9004)); // length < 2, default 9004
        assert_eq!(get_service_port("php"), Some(9004));
    }

    #[test]
    fn test_get_service_port_other() {
        assert_eq!(get_service_port("mailpit"), Some(8025));
        assert_eq!(get_service_port("meilisearch"), Some(7700));
        assert_eq!(get_service_port("unknown"), None);
        assert_eq!(get_service_port("random-service"), None);
    }

    #[test]
    fn test_get_process_names() {
        assert_eq!(get_process_names("nginx"), vec!["nginx.exe"]);
        assert_eq!(get_process_names("apache"), vec!["httpd.exe"]);
        assert_eq!(get_process_names("mariadb"), vec!["mariadbd.exe", "mysqld.exe"]);
        assert_eq!(get_process_names("postgres"), vec!["postgres.exe"]);
        assert_eq!(get_process_names("mongo"), vec!["mongod.exe"]);
        assert_eq!(get_process_names("redis"), vec!["redis-server.exe"]);
        assert_eq!(get_process_names("php-8.4"), vec!["php-cgi.exe"]);
        assert_eq!(get_process_names("mailpit"), vec!["mailpit.exe"]);
        assert_eq!(get_process_names("meilisearch"), vec!["meilisearch.exe"]);
        let empty: Vec<&'static str> = vec![];
        assert_eq!(get_process_names("unknown"), empty);
    }
}
