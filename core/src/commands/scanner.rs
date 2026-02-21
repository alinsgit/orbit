use std::fs;
use tauri::command;
use tauri::AppHandle;
use tauri::Manager;
use crate::services::hidden_command;

#[derive(serde::Serialize)]
pub struct InstalledService {
    pub name: String,         // e.g. "nginx", "php-8.3"
    pub version: String,      // e.g. "1.24.0", "8.3.0"
    pub path: String,         // Absolute path to executable
    pub service_type: String, // "nginx", "php", "mariadb"
    pub port: Option<u16>,    // Actual port from config file
}

#[command]
pub fn get_installed_services(app: AppHandle) -> Result<Vec<InstalledService>, String> {
    let mut services = Vec::new();

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    if !bin_path.exists() {
        return Ok(services);
    }

    // 1. Check Nginx
    let nginx_path = bin_path.join("nginx");
    if nginx_path.exists() {
        let exe_path = nginx_path.join("nginx.exe");
        if exe_path.exists() {
            let version = parse_nginx_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
            let port = parse_nginx_port(&nginx_path);
            services.push(InstalledService {
                name: "nginx".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "nginx".to_string(),
                port,
            });
        }
    }

    // 2. Check MariaDB
    let mariadb_path = bin_path.join("mariadb");
    if mariadb_path.exists() {
        // Try multiple possible locations — prefer mariadbd.exe (newer naming)
        let possible_paths = [
            mariadb_path.join("mariadbd.exe"),             // Newer naming, flat structure
            mariadb_path.join("bin").join("mariadbd.exe"), // Newer naming, standard structure
            mariadb_path.join("mysqld.exe"),               // Legacy naming, flat structure
            mariadb_path.join("bin").join("mysqld.exe"),   // Legacy naming, standard structure
        ];

        for exe_path in possible_paths {
            if exe_path.exists() {
                let version =
                    parse_mariadb_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
                let mariadb_port = parse_mariadb_port(&bin_path);
                services.push(InstalledService {
                    name: "mariadb".to_string(),
                    version,
                    path: exe_path.to_string_lossy().to_string(),
                    service_type: "mariadb".to_string(),
                    port: mariadb_port,
                });
                break;
            }
        }
    }

    // 3. Check PHP Versions
    let php_root = bin_path.join("php");
    if php_root.exists() {
        if let Ok(entries) = fs::read_dir(&php_root) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let version_dir = entry.file_name();
                        let version_str = version_dir.to_string_lossy().to_string();
                        let exe_path = entry.path().join("php-cgi.exe");

                        if exe_path.exists() {
                            let actual_version = parse_php_version(&exe_path)
                                .unwrap_or_else(|_| version_str.clone());
                            // Calculate PHP port from version
                            let php_port = parse_php_port(&actual_version);
                            services.push(InstalledService {
                                name: format!("php-{}", version_str),
                                version: actual_version,
                                path: exe_path.to_string_lossy().to_string(),
                                service_type: "php".to_string(),
                                port: php_port,
                            });
                        }
                    }
                }
            }
        }
    }

    // 4. Check Node.js
    let nodejs_path = bin_path.join("nodejs");
    if nodejs_path.exists() {
        let exe_path = nodejs_path.join("node.exe");
        if exe_path.exists() {
            let version = parse_nodejs_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
            services.push(InstalledService {
                name: "nodejs".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "nodejs".to_string(),
                port: None,
            });
        }
    }

    // 5. Check Python
    let python_path = bin_path.join("python");
    if python_path.exists() {
        let exe_path = python_path.join("python.exe");
        if exe_path.exists() {
            let version = parse_python_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
            services.push(InstalledService {
                name: "python".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "python".to_string(),
                port: None,
            });
        }
    }

    // 6. Check Bun
    let bun_path = bin_path.join("bun");
    if bun_path.exists() {
        let exe_path = bun_path.join("bun.exe");
        if exe_path.exists() {
            let version = parse_bun_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
            services.push(InstalledService {
                name: "bun".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "bun".to_string(),
                port: None,
            });
        }
    }

    // 7. Check Apache
    let apache_path = bin_path.join("apache");
    if apache_path.exists() {
        // Apache Lounge extracts to Apache24/ folder, which after strip_root becomes the root
        let possible_paths = [
            apache_path.join("bin").join("httpd.exe"),  // Standard Apache structure
            apache_path.join("httpd.exe"),              // Flat structure
        ];

        for exe_path in possible_paths {
            if exe_path.exists() {
                let version = parse_apache_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
                let apache_port = parse_apache_port(&bin_path);
                services.push(InstalledService {
                    name: "apache".to_string(),
                    version,
                    path: exe_path.to_string_lossy().to_string(),
                    service_type: "apache".to_string(),
                    port: apache_port,
                });
                break;
            }
        }
    }

    // 8. Check PostgreSQL
    let postgresql_path = bin_path.join("postgresql");
    if postgresql_path.exists() {
        let possible_paths = [
            postgresql_path.join("bin").join("postgres.exe"),
            postgresql_path.join("postgres.exe"),
        ];

        for exe_path in possible_paths {
            if exe_path.exists() {
                let version = parse_postgresql_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
                services.push(InstalledService {
                    name: "postgresql".to_string(),
                    version,
                    path: exe_path.to_string_lossy().to_string(),
                    service_type: "postgresql".to_string(),
                    port: Some(5432),
                });
                break;
            }
        }
    }

    // 9. Check MongoDB
    let mongodb_path = bin_path.join("mongodb");
    if mongodb_path.exists() {
        let possible_paths = [
            mongodb_path.join("bin").join("mongod.exe"),
            mongodb_path.join("mongod.exe"),
        ];

        for exe_path in possible_paths {
            if exe_path.exists() {
                let version = parse_mongodb_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
                services.push(InstalledService {
                    name: "mongodb".to_string(),
                    version,
                    path: exe_path.to_string_lossy().to_string(),
                    service_type: "mongodb".to_string(),
                    port: Some(27017),
                });
                break;
            }
        }
    }

    // 10. Check Redis
    let redis_path = bin_path.join("redis");
    if redis_path.exists() {
        let exe_path = redis_path.join("redis-server.exe");
        if exe_path.exists() {
            let version = parse_redis_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
            services.push(InstalledService {
                name: "redis".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "redis".to_string(),
                port: Some(6379),
            });
        }
    }

    // 11. Check Go
    let go_path = bin_path.join("go");
    if go_path.exists() {
        let possible_paths = [
            go_path.join("bin").join("go.exe"),
            go_path.join("go.exe"),
        ];

        for exe_path in possible_paths {
            if exe_path.exists() {
                let version = parse_go_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
                services.push(InstalledService {
                    name: "go".to_string(),
                    version,
                    path: exe_path.to_string_lossy().to_string(),
                    service_type: "go".to_string(),
                    port: None,
                });
                break;
            }
        }
    }

    // 12. Check Deno
    let deno_path = bin_path.join("deno");
    if deno_path.exists() {
        let exe_path = deno_path.join("deno.exe");
        if exe_path.exists() {
            let version = parse_deno_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
            services.push(InstalledService {
                name: "deno".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "deno".to_string(),
                port: None,
            });
        }
    }

    Ok(services)
}

fn parse_nginx_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("-v")
        .output()
        .map_err(|e| e.to_string())?;

    // nginx -v writes to stderr, not stdout
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    // Look for "nginx/x.x.x" pattern
    if let Some(pos) = combined.find("nginx/") {
        let version_start = pos + 6;
        let version_end = combined[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(combined.len());
        let version = &combined[version_start..version_end];
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse nginx version".to_string())
}

fn parse_php_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("-v")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Look for "PHP x.x.x" pattern
    if let Some(pos) = combined.find("PHP ") {
        let version_start = pos + 4;
        let version_end = combined[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(combined.len());
        let version = &combined[version_start..version_end];
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse PHP version".to_string())
}

fn parse_mariadb_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Look for version pattern like "11.4.2-MariaDB" or "Ver 11.4.2"
    // Common output: "mysqld  Ver 11.4.2-MariaDB for Win64 on AMD64"
    if let Some(pos) = combined.find("Ver ") {
        let version_start = pos + 4;
        let version_end = combined[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(combined.len());
        let version = &combined[version_start..version_end];
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    // Alternative: look for pattern like "11.4.2-MariaDB"
    for word in combined.split_whitespace() {
        if word.contains("MariaDB") || word.contains("-mariadb") {
            let version = word.split('-').next().unwrap_or("");
            if !version.is_empty() && version.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                return Ok(version.to_string());
            }
        }
    }

    Err("Could not parse MariaDB version".to_string())
}

fn parse_nodejs_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output is like "v22.16.0"
    let version = stdout.trim().trim_start_matches('v');
    if !version.is_empty() {
        return Ok(version.to_string());
    }

    Err("Could not parse Node.js version".to_string())
}

fn parse_python_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Output is like "Python 3.13.2"
    if let Some(pos) = combined.find("Python ") {
        let version_start = pos + 7;
        let version_end = combined[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(combined.len());
        let version = combined[version_start..version_end].trim();
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse Python version".to_string())
}

fn parse_bun_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output is like "1.2.4"
    let version = stdout.trim();
    if !version.is_empty() && version.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Ok(version.to_string());
    }

    Err("Could not parse Bun version".to_string())
}

/// Parse listen port from nginx.conf
fn parse_nginx_port(nginx_root: &std::path::Path) -> Option<u16> {
    let conf_path = nginx_root.join("conf").join("nginx.conf");
    let content = fs::read_to_string(conf_path).ok()?;
    // Find first "listen <port>;" outside of comments
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(pos) = trimmed.find("listen") {
            let after = trimmed[pos + 6..].trim();
            // Extract port number (first numeric token)
            let port_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(port) = port_str.parse::<u16>() {
                return Some(port);
            }
        }
    }
    None
}

/// Parse port from MariaDB my.ini
fn parse_mariadb_port(bin_path: &std::path::Path) -> Option<u16> {
    let data_dir = bin_path.join("mariadb").join("data");
    let ini_path = data_dir.join("my.ini");
    let content = fs::read_to_string(ini_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with("port") {
            // port = 3306 or port=3306
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(port) = val.trim().parse::<u16>() {
                    return Some(port);
                }
            }
        }
    }
    None
}

/// Calculate PHP FastCGI port from version string
fn parse_php_port(version: &str) -> Option<u16> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        if let (Ok(major), Ok(minor)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
            return Some(9000 + (major * 10 + minor) - 80);
        }
    }
    None
}

/// Parse Listen port from Apache httpd.conf
fn parse_apache_port(bin_path: &std::path::Path) -> Option<u16> {
    let conf_path = bin_path.join("apache").join("conf").join("httpd.conf");
    let content = fs::read_to_string(conf_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("Listen") {
            let after = trimmed[6..].trim();
            let port_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(port) = port_str.parse::<u16>() {
                return Some(port);
            }
        }
    }
    None
}

fn parse_postgresql_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output: "postgres (PostgreSQL) 16.4"
    if let Some(pos) = stdout.find("PostgreSQL) ") {
        let version_start = pos + 12;
        let version_end = stdout[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(stdout.len());
        let version = stdout[version_start..version_end].trim();
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse PostgreSQL version".to_string())
}

fn parse_mongodb_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output: "db version v7.0.14"
    if let Some(pos) = stdout.find("db version v") {
        let version_start = pos + 12;
        let version_end = stdout[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(stdout.len());
        let version = stdout[version_start..version_end].trim();
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse MongoDB version".to_string())
}

fn parse_redis_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output: "Redis server v=7.2.4 sha=..."
    if let Some(pos) = stdout.find("v=") {
        let version_start = pos + 2;
        let version_end = stdout[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(stdout.len());
        let version = stdout[version_start..version_end].trim();
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse Redis version".to_string())
}

fn parse_go_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output: "go version go1.22.5 windows/amd64"
    if let Some(pos) = stdout.find("go1.") {
        let version_start = pos + 2; // skip "go", keep "1.22.5"
        let version_end = stdout[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(stdout.len());
        let version = stdout[version_start..version_end].trim();
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse Go version".to_string())
}

fn parse_deno_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output: "deno 1.42.4 (...)"
    if let Some(pos) = stdout.find("deno ") {
        let version_start = pos + 5;
        let version_end = stdout[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(stdout.len());
        let version = stdout[version_start..version_end].trim();
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse Deno version".to_string())
}

/// Parse port from PostgreSQL postgresql.conf
#[allow(dead_code)]
fn parse_postgresql_port(bin_path: &std::path::Path) -> Option<u16> {
    let data_dir = bin_path.join("data").join("postgres");
    let conf_path = data_dir.join("postgresql.conf");
    let content = fs::read_to_string(conf_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("port") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let val = val.trim().trim_matches(|c: char| !c.is_ascii_digit());
                if let Ok(port) = val.parse::<u16>() {
                    return Some(port);
                }
            }
        }
    }
    None
}

fn parse_apache_version(exe_path: &std::path::PathBuf) -> Result<String, String> {
    let output = hidden_command(exe_path)
        .arg("-v")
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Output is like "Server version: Apache/2.4.62 (Win64)"
    if let Some(pos) = combined.find("Apache/") {
        let version_start = pos + 7;
        let version_end = combined[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(combined.len());
        let version = &combined[version_start..version_end];
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    Err("Could not parse Apache version".to_string())
}
