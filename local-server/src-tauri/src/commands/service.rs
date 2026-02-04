use crate::services::php_registry::PhpRegistry;
use crate::services::process::{ServiceManager, ServiceType};
use tauri::{command, AppHandle, Manager, State};

/// Parse PHP version from service name (e.g., "php-8.4" -> "8.4")
fn parse_php_version_string(name: &str) -> String {
    // Try to extract version from name like "php-8.4" or "php-8.5"
    if let Some(version_str) = name.strip_prefix("php-") {
        version_str.to_string()
    } else if name.contains("php") {
        // Try to extract version pattern like "8.4"
        let parts: Vec<&str> = name.split(|c: char| !c.is_ascii_digit() && c != '.').collect();
        for part in parts {
            if part.contains('.') && part.len() >= 3 {
                return part.to_string();
            }
        }
        "8.4".to_string()
    } else {
        "8.4".to_string()
    }
}

/// Parse PHP version to numeric (e.g., "8.4" -> 84)
fn parse_php_version_numeric(name: &str) -> u32 {
    let version = parse_php_version_string(name);
    let cleaned: String = version.chars().filter(|c| c.is_ascii_digit()).collect();
    cleaned.parse().unwrap_or(84)
}

/// Get PHP port from registry or calculate
fn get_php_port(app: &AppHandle, version: &str) -> u16 {
    if let Ok(registry) = PhpRegistry::load(app) {
        registry.get_or_calculate_port(version)
    } else {
        PhpRegistry::calculate_port(version)
    }
}

#[command]
pub fn start_service(
    app: AppHandle,
    state: State<'_, ServiceManager>,
    name: String,
    bin_path: String,
) -> Result<String, String> {
    // Determine service type based on name
    let is_php = name.contains("php");
    let php_version_str = if is_php {
        parse_php_version_string(&name)
    } else {
        String::new()
    };

    let service_type = if name.contains("nginx") {
        ServiceType::Nginx
    } else if is_php {
        let version = parse_php_version_numeric(&name);
        ServiceType::Php(version)
    } else if name.contains("apache") || name.contains("httpd") {
        ServiceType::Apache
    } else if name.contains("node") {
        ServiceType::NodeJs
    } else if name.contains("python") {
        ServiceType::Python
    } else if name.contains("bun") {
        ServiceType::Bun
    } else {
        ServiceType::MariaDB
    };

    // Build args based on service type
    let bin_path_buf = std::path::PathBuf::from(&bin_path);

    let args: Vec<String> = match service_type {
        ServiceType::Nginx => vec![],
        ServiceType::Php(_) => {
            // Get port from registry
            let port = get_php_port(&app, &php_version_str);
            vec!["-b".to_string(), format!("127.0.0.1:{}", port)]
        }
        ServiceType::MariaDB => {
            use crate::services::mariadb::MariaDBManager;

            let app_bin = app
                .path()
                .app_local_data_dir()
                .map_err(|e| e.to_string())?
                .join("bin");

            let mariadb_root = app_bin.join("mariadb");
            let data_dir = app_bin.join("data").join("mariadb");

            // Auto-initialize if needed
            if !data_dir.join("mysql").exists() {
                log::info!("MariaDB not initialized, auto-initializing...");
                MariaDBManager::initialize(&mariadb_root, &data_dir, "root")?;
            }

            let config_path = data_dir.join("my.ini");

            // --defaults-file MUST be the first argument for MariaDB
            let mut args: Vec<String> = Vec::new();

            if config_path.exists() {
                args.push(format!("--defaults-file={}", config_path.display()));
            }

            args.push("--console".to_string());
            args.push(format!("--datadir={}", data_dir.display()));

            args
        }
        ServiceType::Apache => {
            // Apache httpd doesn't need special args on Windows
            vec![]
        }
        ServiceType::NodeJs | ServiceType::Python | ServiceType::Bun => {
            // These are typically not background services
            vec![]
        }
    };

    // Convert Vec<String> to Vec<&str> for the function call
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match state.start_with_name(name.clone(), service_type, &bin_path, &args_refs) {
        Ok(pid) => {
            // Update PHP registry if it's a PHP service
            if is_php {
                if let Ok(mut registry) = PhpRegistry::load(&app) {
                    // Register if not exists
                    if let Some(parent) = bin_path_buf.parent() {
                        registry.register_php(&php_version_str, parent.to_string_lossy().as_ref());
                    }
                    registry.mark_running(&php_version_str, pid);
                    let _ = registry.save(&app);
                }
            }
            Ok(format!("Service {} started with PID {}", name, pid))
        }
        Err(e) => Err(e),
    }
}

#[command]
pub fn stop_service(
    app: AppHandle,
    state: State<'_, ServiceManager>,
    name: String,
) -> Result<String, String> {
    match state.stop(&name) {
        Ok(_) => {
            // Update PHP registry if it's a PHP service
            if name.contains("php") {
                let php_version_str = parse_php_version_string(&name);
                if let Ok(mut registry) = PhpRegistry::load(&app) {
                    registry.mark_stopped(&php_version_str);
                    let _ = registry.save(&app);
                }
            }
            Ok(format!("Service {} stopped", name))
        }
        Err(e) => Err(e),
    }
}

#[command]
pub fn get_service_status(
    state: State<'_, ServiceManager>,
    name: String,
) -> Result<String, String> {
    match state.get_status(&name) {
        Some(status) => Ok(status),
        None => Err("Service not found".to_string()),
    }
}

#[command]
pub fn initialize_mariadb(app: AppHandle, _root_password: String) -> Result<String, String> {
    use crate::services::mariadb::MariaDBManager;

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    let mariadb_path = bin_path.join("mariadb");
    let data_path = bin_path.join("data").join("mariadb");

    MariaDBManager::initialize(&mariadb_path, &data_path, "root")?;

    Ok("MariaDB initialized successfully".to_string())
}

#[command]
pub fn uninstall_service(
    state: State<'_, ServiceManager>,
    name: String,
    _service_type: String,
    path: String,
) -> Result<String, String> {
    use std::path::Path;

    // Try to stop the service, but don't fail if it's not running
    let _ = state.stop(&name);

    let service_path = Path::new(&path);
    if let Some(parent) = service_path.parent() {
        if parent.exists() {
            std::fs::remove_dir_all(parent)
                .map_err(|e| format!("Failed to remove service directory: {}", e))?;
        }
    }

    Ok(format!("Service {} uninstalled successfully", name))
}

#[command]
pub fn assign_php_port(php_version: u32, start_port: u16) -> Result<u16, String> {
    let offset = (php_version - 80) / 1;
    let port = start_port + offset as u16;

    Ok(port)
}

#[command]
pub fn check_port_conflict(port: u16) -> Result<bool, String> {
    use std::net::TcpListener;

    match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(_) => Ok(false),
        Err(_) => Ok(true),
    }
}

#[command]
pub fn reload_service(app: AppHandle, name: String) -> Result<String, String> {
    use crate::services::nginx::NginxManager;
    use std::process::Command;

    if name.contains("nginx") {
        // Use existing nginx reload
        NginxManager::reload(&app)
    } else if name.contains("apache") || name.contains("httpd") {
        // Apache graceful restart
        let bin_path = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("apache");

        let httpd_path = bin_path.join("bin").join("httpd.exe");

        if !httpd_path.exists() {
            return Err("Apache httpd.exe not found".to_string());
        }

        let mut cmd = Command::new(&httpd_path);
        cmd.current_dir(bin_path)
            .args(["-k", "graceful"]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd.output()
            .map_err(|e| format!("Failed to reload Apache: {}", e))?;

        if output.status.success() {
            Ok("Apache configuration reloaded".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Apache reload failed: {}", stderr))
        }
    } else {
        Err(format!("Service '{}' does not support reload. Use restart instead.", name))
    }
}
