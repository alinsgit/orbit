//! PECL Extension Manager - Multiplatform
//! Installs PHP extensions dynamically using PECL REST API
//! - Linux/macOS: Uses `pecl install` command
//! - Windows: Downloads DLL from windows.php.net or alternative sources

#[cfg(target_os = "windows")]
use crate::services::download::download_file;
use crate::services::validation::validate_php_version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "windows"))]
use std::process::Command;
use tauri::{command, AppHandle, Manager};

/// Information about a PECL extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeclExtension {
    pub name: String,
    pub version: String,
    pub description: String,
    pub download_url: Option<String>,
    pub installed: bool,
    pub enabled: bool,
    pub category: String,
}

/// Get the list of available extensions from PECL + installed status
#[command]
pub async fn get_available_extensions(
    app: AppHandle,
    php_version: String,
) -> Result<Vec<PeclExtension>, String> {
    validate_php_version(&php_version).map_err(|e| e.to_string())?;

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(&php_version);

    let ext_dir = bin_path.join("ext");
    let ini_path = bin_path.join("php.ini");

    // Get currently installed extensions by scanning ext directory
    let installed_exts: Vec<String> = if ext_dir.exists() {
        fs::read_dir(&ext_dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        // Handle both php_xxx.dll (Windows) and xxx.so (Linux/macOS)
                        if name.ends_with(".dll") || name.ends_with(".so") {
                            let ext_name = name
                                .strip_prefix("php_")
                                .unwrap_or(&name)
                                .strip_suffix(".dll")
                                .or_else(|| name.strip_suffix(".so"))
                                .unwrap_or(&name)
                                .to_string();
                            Some(ext_name)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // Read php.ini to check enabled status
    let ini_content = if ini_path.exists() {
        fs::read_to_string(&ini_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Use hardcoded extension info for fast loading (no slow HTTP calls)
    let mut result: Vec<PeclExtension> = Vec::new();

    for (ext_name, description, category) in get_extension_info() {
        let is_installed = installed_exts.iter().any(|e| e == ext_name);
        let is_enabled = is_installed
            && (ini_content.contains(&format!("extension={ext_name}"))
                || ini_content.contains(&format!("extension={ext_name}.so"))
                || ini_content.contains(&format!("extension=php_{ext_name}.dll")))
            && !ini_content.contains(&format!(";extension={ext_name}"));

        result.push(PeclExtension {
            name: ext_name.to_string(),
            version: "latest".to_string(),
            description: description.to_string(),
            download_url: Some(format!("pecl://{ext_name}")),
            installed: is_installed,
            enabled: is_enabled,
            category: category.to_string(),
        });
    }

    // Sort: installed first, then alphabetically
    result.sort_by(|a, b| {
        match (a.installed, b.installed) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(result)
}

/// Get extension info - hardcoded for fast loading
/// Only includes extensions that work without external dependencies
fn get_extension_info() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("redis", "Redis client for PHP with clustering support", "Caching"),
        ("apcu", "APC User Cache - in-memory key-value store", "Caching"),
        ("xdebug", "Debugging and profiling extension", "Development"),
        ("mongodb", "MongoDB driver for PHP", "Database"),
        ("igbinary", "Binary serializer for PHP", "Serialization"),
        ("protobuf", "Protocol Buffers serialization", "Serialization"),
        ("xlswriter", "Fast Excel file writer", "Office"),
        ("ds", "Efficient data structures (Deque, Map, Set)", "Data"),
        ("ast", "Abstract Syntax Tree extension", "Development"),
        ("pcov", "Code coverage driver (faster than Xdebug)", "Testing"),
        ("uuid", "UUID generation and parsing", "Utilities"),
        ("sodium", "Modern cryptography (NaCl)", "Security"),
    ]
}

/// Fetch available versions for an extension from PECL REST API
#[allow(dead_code)]
async fn fetch_pecl_versions(extension_name: &str) -> Vec<String> {
    let url = format!(
        "https://pecl.php.net/rest/r/{}/allreleases.xml",
        extension_name.to_lowercase()
    );

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    if !response.status().is_success() {
        return vec![];
    }

    let text = match response.text().await {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    // Parse versions from XML - look for <v>X.Y.Z</v> tags
    let mut versions: Vec<String> = Vec::new();
    let mut remaining = text.as_str();
    
    while let Some(start) = remaining.find("<v>") {
        let content_start = start + 3;
        if let Some(end) = remaining[content_start..].find("</v>") {
            let version = remaining[content_start..content_start + end].trim();
            // Only take stable versions (no alpha, beta, RC)
            if !version.contains("alpha") && !version.contains("beta") && !version.contains("RC") && !version.contains("rc") {
                versions.push(version.to_string());
            }
            remaining = &remaining[content_start + end + 4..];
        } else {
            break;
        }
    }

    // Return first 5 stable versions (most recent first)
    versions.truncate(5);
    versions
}

/// Install a PECL extension - multiplatform
#[command]
pub async fn install_pecl_extension(
    app: AppHandle,
    php_version: String,
    extension_name: String,
) -> Result<String, String> {
    validate_php_version(&php_version).map_err(|e| e.to_string())?;

    // Validate extension name
    if extension_name.is_empty()
        || !extension_name.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return Err("Invalid extension name".to_string());
    }

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(&php_version);

    // Detect platform
    #[cfg(target_os = "windows")]
    {
        install_extension_windows(&app, &bin_path, &php_version, &extension_name).await
    }

    #[cfg(not(target_os = "windows"))]
    {
        install_extension_unix(&bin_path, &php_version, &extension_name).await
    }
}

/// Install extension on Windows - download DLL
#[cfg(target_os = "windows")]
async fn install_extension_windows(
    app: &AppHandle,
    bin_path: &Path,
    php_version: &str,
    extension_name: &str,
) -> Result<String, String> {
    let ext_dir = bin_path.join("ext");
    let dll_path = ext_dir.join(format!("php_{extension_name}.dll"));

    if dll_path.exists() {
        return Err(format!("Extension {extension_name} is already installed"));
    }

    // Parse PHP version
    let version_parts: Vec<&str> = php_version.split('.').collect();
    let php_major_minor = if version_parts.len() >= 2 {
        format!("{}.{}", version_parts[0], version_parts[1])
    } else {
        return Err("Invalid PHP version format".to_string());
    };

    // Fetch available versions dynamically from PECL API
    log::info!("Fetching available versions for {extension_name} from PECL API");
    let dynamic_versions = fetch_pecl_versions(extension_name).await;
    
    // Try multiple download sources with dynamic versions
    let download_urls = get_windows_download_urls(extension_name, &php_major_minor, &dynamic_versions);
    
    if download_urls.is_empty() {
        return Err(format!(
            "No Windows DLL available for {extension_name} on PHP {php_major_minor}. Consider using pecl on WSL or compiling from source."
        ));
    }

    // Create temp directory
    let temp_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("temp");
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {e}"))?;

    // Try each download URL
    let mut last_error = String::new();
    for url in download_urls {
        log::info!("Trying to download {extension_name} from {url}");
        
        let is_direct_dll = url.ends_with(".dll");
        let temp_file = temp_dir.join(if is_direct_dll {
            format!("php_{extension_name}.dll")
        } else {
            format!("{extension_name}.zip")
        });

        match download_file(&url, &temp_file).await {
            Ok(_) => {
                if is_direct_dll {
                    // Direct DLL - just copy
                    fs::copy(&temp_file, &dll_path)
                        .map_err(|e| format!("Failed to copy DLL: {e}"))?;
                    fs::remove_file(&temp_file).ok();
                } else {
                    // ZIP file - extract
                    let extract_dir = temp_dir.join(extension_name);
                    if extract_dir.exists() {
                        fs::remove_dir_all(&extract_dir).ok();
                    }
                    fs::create_dir_all(&extract_dir)
                        .map_err(|e| format!("Failed to create extract dir: {e}"))?;

                    crate::services::download::extract_zip(&temp_file, &extract_dir)?;

                    // Find the DLL file
                    let dll_name = format!("php_{extension_name}.dll");
                    if let Ok(source_dll) = find_dll_recursive(&extract_dir, &dll_name) {
                        fs::copy(&source_dll, &dll_path)
                            .map_err(|e| format!("Failed to copy DLL: {e}"))?;
                    } else {
                        fs::remove_file(&temp_file).ok();
                        fs::remove_dir_all(&extract_dir).ok();
                        last_error = "DLL not found in archive".to_string();
                        continue;
                    }

                    // Cleanup
                    fs::remove_file(&temp_file).ok();
                    fs::remove_dir_all(&extract_dir).ok();
                }

                log::info!("Extension {extension_name} installed successfully");
                return Ok(format!(
                    "Extension {extension_name} installed. Add 'extension={extension_name}' to php.ini to enable."
                ));
            }
            Err(e) => {
                last_error = e;
                continue;
            }
        }
    }

    Err(format!(
        "Failed to install {extension_name}: {last_error}. Windows PECL builds may not be available for PHP {php_version}."
    ))
}

/// Get Windows download URLs for an extension
/// Uses multiple sources: downloads.php.net (primary), xdebug.org, windows.php.net
/// Now accepts dynamic versions from PECL API
#[cfg(target_os = "windows")]
fn get_windows_download_urls(extension_name: &str, php_version: &str, dynamic_versions: &[String]) -> Vec<String> {
    let vs_version = match php_version {
        "8.4" | "8.5" => "vs17",
        "8.2" | "8.3" => "vs16",
        "8.0" | "8.1" => "vs16",
        "7.4" => "vc15",
        _ => "vs16",
    };

    let mut urls = Vec::new();

    // Special case for xdebug - xdebug.org has latest builds for all PHP versions
    if extension_name == "xdebug" {
        // Use dynamic versions if available, otherwise fallback
        let xdebug_versions: Vec<&str> = if !dynamic_versions.is_empty() {
            dynamic_versions.iter().take(3).map(|s| s.as_str()).collect()
        } else {
            vec!["3.4.0", "3.3.2", "3.3.1"]
        };
        
        for xdebug_ver in xdebug_versions {
            urls.push(format!(
                "https://xdebug.org/files/php_xdebug-{xdebug_ver}-{php_version}-nts-{vs_version}-x64.dll"
            ));
        }
        return urls;
    }

    // Use dynamic versions from PECL API first, then fallback to hardcoded
    let versions: Vec<&str> = if !dynamic_versions.is_empty() {
        log::info!("Using dynamic versions from PECL API: {dynamic_versions:?}");
        dynamic_versions.iter().take(5).map(|s| s.as_str()).collect()
    } else {
        // Fallback to known working versions if API fails
        log::info!("PECL API unavailable, using fallback versions");
        match extension_name {
            "redis" => vec!["6.1.0", "6.0.2", "5.3.7"],
            "apcu" => vec!["5.1.24", "5.1.23", "5.1.21"],
            "igbinary" => vec!["3.2.16", "3.2.15", "3.2.7"],
            "mongodb" => vec!["1.20.0", "1.19.0", "1.18.0"],
            "protobuf" => vec!["4.28.0", "3.25.0"],
            "xlswriter" => vec!["1.5.8", "1.5.5"],
            "ds" => vec!["1.5.0", "1.4.0"],
            "ast" => vec!["1.1.2", "1.1.1"],
            "pcov" => vec!["1.0.12", "1.0.11"],
            "uuid" => vec!["1.2.1", "1.2.0"],
            _ => vec![],
        }
    };

    // Primary source: downloads.php.net/~windows/pecl/releases/
    // This source has PHP 8.2, 8.3, 8.4 builds!
    for ver in &versions {
        urls.push(format!(
            "https://downloads.php.net/~windows/pecl/releases/{extension_name}/{ver}/php_{extension_name}-{ver}-{php_version}-nts-{vs_version}-x64.zip"
        ));
    }

    // Fallback: windows.php.net (older builds, PHP 8.0-8.1)
    for ver in &versions {
        urls.push(format!(
            "https://windows.php.net/downloads/pecl/releases/{extension_name}/{ver}/php_{extension_name}-{ver}-{php_version}-nts-{vs_version}-x64.zip"
        ));
    }

    urls
}

/// Install extension on Linux/macOS using pecl command
#[cfg(not(target_os = "windows"))]
async fn install_extension_unix(
    bin_path: &Path,
    _php_version: &str,
    extension_name: &str,
) -> Result<String, String> {
    let php_exe = bin_path.join("bin").join("php");
    let pecl_exe = bin_path.join("bin").join("pecl");
    #[allow(unused)]
    let phpize_exe = bin_path.join("bin").join("phpize");

    // Check if pecl exists
    if !pecl_exe.exists() {
        return Err(format!(
            "pecl not found at {:?}. Please install php-pear or php-dev package.",
            pecl_exe
        ));
    }

    log::info!("Installing {} using pecl", extension_name);

    // Run pecl install
    let output = Command::new(&pecl_exe)
        .arg("install")
        .arg(extension_name)
        .env("PHP_PEAR_PHP_BIN", &php_exe)
        .output()
        .map_err(|e| format!("Failed to run pecl: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::info!("pecl install output: {}", stdout);

        Ok(format!(
            "Extension {} installed. Add 'extension={}.so' to php.ini to enable.",
            extension_name, extension_name
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "pecl install failed: {}\n{}",
            stderr, stdout
        ))
    }
}

/// Find a DLL file recursively in a directory
#[cfg(target_os = "windows")]
fn find_dll_recursive(dir: &PathBuf, dll_name: &str) -> Result<PathBuf, String> {
    if !dir.exists() {
        return Err(format!("Directory does not exist: {dir:?}"));
    }

    for entry in fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy() == dll_name {
                    return Ok(path);
                }
            }
        } else if path.is_dir() {
            if let Ok(found) = find_dll_recursive(&path, dll_name) {
                return Ok(found);
            }
        }
    }

    Err(format!("DLL {dll_name} not found in directory"))
}

/// Uninstall a PECL extension
#[command]
pub async fn uninstall_pecl_extension(
    app: AppHandle,
    php_version: String,
    extension_name: String,
) -> Result<String, String> {
    validate_php_version(&php_version).map_err(|e| e.to_string())?;

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(&php_version);

    let ext_dir = bin_path.join("ext");
    
    // Try both .dll and .so extensions
    let dll_path = ext_dir.join(format!("php_{extension_name}.dll"));
    let so_path = ext_dir.join(format!("{extension_name}.so"));

    let ext_path = if dll_path.exists() {
        dll_path
    } else if so_path.exists() {
        so_path
    } else {
        return Err(format!("Extension {extension_name} is not installed"));
    };

    // Disable in php.ini first
    let ini_path = bin_path.join("php.ini");
    if ini_path.exists() {
        let content = fs::read_to_string(&ini_path)
            .map_err(|e| format!("Failed to read php.ini: {e}"))?;

        // Comment out the extension line
        let patterns = [
            format!("extension={extension_name}"),
            format!("extension={extension_name}.so"),
            format!("extension=php_{extension_name}.dll"),
        ];

        let mut new_content = content.clone();
        for pattern in &patterns {
            if new_content.contains(pattern) && !new_content.contains(&format!(";{pattern}")) {
                new_content = new_content.replace(pattern, &format!(";{pattern}"));
            }
        }

        if new_content != content {
            fs::write(&ini_path, new_content)
                .map_err(|e| format!("Failed to update php.ini: {e}"))?;
        }
    }

    // Delete the extension file
    fs::remove_file(&ext_path).map_err(|e| format!("Failed to delete extension: {e}"))?;

    log::info!("Extension {extension_name} uninstalled");

    Ok(format!("Extension {extension_name} uninstalled successfully"))
}

/// Search for extensions in PECL
#[command]
pub async fn search_pecl_extensions(
    query: String,
    _php_version: String,
) -> Result<Vec<PeclExtension>, String> {
    // Search within available extensions
    let query_lower = query.to_lowercase();
    
    let results: Vec<PeclExtension> = get_extension_info()
        .iter()
        .filter(|(name, desc, _)| {
            name.to_lowercase().contains(&query_lower) || 
            desc.to_lowercase().contains(&query_lower)
        })
        .map(|(name, description, category)| PeclExtension {
            name: name.to_string(),
            version: "latest".to_string(),
            description: description.to_string(),
            download_url: Some(format!("pecl://{name}")),
            installed: false,
            enabled: false,
            category: category.to_string(),
        })
        .collect();

    Ok(results)
}
