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
            services.push(InstalledService {
                name: "nginx".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "nginx".to_string(),
            });
        }
    }

    // 2. Check MariaDB
    let mariadb_path = bin_path.join("mariadb");
    if mariadb_path.exists() {
        // Try multiple possible locations for mysqld.exe
        let possible_paths = [
            mariadb_path.join("bin").join("mysqld.exe"),  // Standard structure
            mariadb_path.join("mysqld.exe"),              // Flat structure (after strip_root)
        ];

        for exe_path in possible_paths {
            if exe_path.exists() {
                let version =
                    parse_mariadb_version(&exe_path).unwrap_or_else(|_| "unknown".to_string());
                services.push(InstalledService {
                    name: "mariadb".to_string(),
                    version,
                    path: exe_path.to_string_lossy().to_string(),
                    service_type: "mariadb".to_string(),
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
                            services.push(InstalledService {
                                name: format!("php-{}", version_str),
                                version: actual_version,
                                path: exe_path.to_string_lossy().to_string(),
                                service_type: "php".to_string(),
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
                services.push(InstalledService {
                    name: "apache".to_string(),
                    version,
                    path: exe_path.to_string_lossy().to_string(),
                    service_type: "apache".to_string(),
                });
                break;
            }
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
