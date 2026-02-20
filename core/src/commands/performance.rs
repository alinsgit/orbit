use std::fs;
use tauri::{command, AppHandle, Manager};
use crate::services::nginx::NginxManager;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PerformanceStatus {
    pub opcache_enabled: bool,
    pub opcache_memory: String,
    pub nginx_gzip_enabled: bool,
    pub nginx_gzip_level: u8,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct OpcacheConfig {
    pub enabled: bool,
    pub memory: String,           // e.g., "128"
    pub max_files: String,        // e.g., "10000"
    pub validate_timestamps: bool,
    pub revalidate_freq: String,  // e.g., "2"
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct NginxGzipConfig {
    pub enabled: bool,
    pub level: u8,  // 1-9
    pub min_length: String,
    pub types: Vec<String>,
}

/// Get current performance status
#[command]
pub fn get_performance_status(app: AppHandle, php_version: Option<String>) -> Result<PerformanceStatus, String> {
    // Check OPcache status (if PHP version provided)
    let (opcache_enabled, opcache_memory) = if let Some(version) = php_version {
        get_opcache_status(&app, &version)?
    } else {
        (false, "0".to_string())
    };

    // Check Nginx gzip status
    let (nginx_gzip_enabled, nginx_gzip_level) = get_nginx_gzip_status(&app)?;

    Ok(PerformanceStatus {
        opcache_enabled,
        opcache_memory,
        nginx_gzip_enabled,
        nginx_gzip_level,
    })
}

/// Get OPcache configuration
#[command]
pub fn get_opcache_config(app: AppHandle, version: String) -> Result<OpcacheConfig, String> {
    let ini_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(&version)
        .join("php.ini");

    if !ini_path.exists() {
        return Ok(OpcacheConfig {
            enabled: false,
            memory: "128".to_string(),
            max_files: "10000".to_string(),
            validate_timestamps: true,
            revalidate_freq: "2".to_string(),
        });
    }

    let content = fs::read_to_string(&ini_path)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    let enabled = content.contains("opcache.enable=1") ||
                  content.contains("opcache.enable = 1");

    let memory = parse_ini_value(&content, "opcache.memory_consumption")
        .unwrap_or_else(|| "128".to_string());

    let max_files = parse_ini_value(&content, "opcache.max_accelerated_files")
        .unwrap_or_else(|| "10000".to_string());

    let validate = parse_ini_value(&content, "opcache.validate_timestamps")
        .map(|v| v == "1" || v.to_lowercase() == "on")
        .unwrap_or(true);

    let revalidate = parse_ini_value(&content, "opcache.revalidate_freq")
        .unwrap_or_else(|| "2".to_string());

    Ok(OpcacheConfig {
        enabled,
        memory,
        max_files,
        validate_timestamps: validate,
        revalidate_freq: revalidate,
    })
}

/// Update OPcache configuration
#[command]
pub fn set_opcache_config(app: AppHandle, version: String, config: OpcacheConfig) -> Result<String, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(&version);

    let ini_path = bin_path.join("php.ini");

    if !ini_path.exists() {
        return Err("php.ini not found".to_string());
    }

    let mut content = fs::read_to_string(&ini_path)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    // Check if extension=opcache exists, if not add it
    if !content.contains("extension=opcache") && !content.contains("zend_extension=opcache") {
        // Add opcache extension
        content.push_str("\n; OPcache Extension\nzend_extension=opcache\n");
    }

    // Remove existing opcache settings
    let lines: Vec<&str> = content.lines().collect();
    let filtered: Vec<&str> = lines.iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("opcache.") && !trimmed.starts_with(";opcache.")
        })
        .cloned()
        .collect();

    content = filtered.join("\n");

    // Add OPcache configuration block
    let opcache_block = format!(r#"
; OPcache Settings
opcache.enable={}
opcache.enable_cli=0
opcache.memory_consumption={}
opcache.interned_strings_buffer=16
opcache.max_accelerated_files={}
opcache.validate_timestamps={}
opcache.revalidate_freq={}
opcache.save_comments=1
opcache.fast_shutdown=1
"#,
        if config.enabled { "1" } else { "0" },
        config.memory,
        config.max_files,
        if config.validate_timestamps { "1" } else { "0" },
        config.revalidate_freq
    );

    content.push_str(&opcache_block);

    fs::write(&ini_path, content)
        .map_err(|e| format!("Failed to write php.ini: {}", e))?;

    Ok("OPcache configuration updated. Restart PHP to apply changes.".to_string())
}

/// Get Nginx gzip configuration
#[command]
pub fn get_nginx_gzip_config(app: AppHandle) -> Result<NginxGzipConfig, String> {
    let config_dir = NginxManager::get_config_dir(&app)?;
    let nginx_conf = config_dir.join("nginx.conf");

    if !nginx_conf.exists() {
        return Ok(NginxGzipConfig {
            enabled: false,
            level: 6,
            min_length: "1000".to_string(),
            types: default_gzip_types(),
        });
    }

    let content = fs::read_to_string(&nginx_conf)
        .map_err(|e| format!("Failed to read nginx.conf: {}", e))?;

    // Check if gzip is enabled (gzip on; not commented)
    let enabled = content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("gzip") && trimmed.contains("on") && !trimmed.starts_with("#")
    });

    // Parse gzip_comp_level
    let level = content.lines()
        .find(|line| line.trim().starts_with("gzip_comp_level"))
        .and_then(|line| {
            line.split_whitespace()
                .nth(1)
                .and_then(|v| v.trim_end_matches(';').parse().ok())
        })
        .unwrap_or(6);

    // Parse gzip_min_length
    let min_length = content.lines()
        .find(|line| line.trim().starts_with("gzip_min_length"))
        .and_then(|line| {
            line.split_whitespace()
                .nth(1)
                .map(|v| v.trim_end_matches(';').to_string())
        })
        .unwrap_or_else(|| "1000".to_string());

    Ok(NginxGzipConfig {
        enabled,
        level,
        min_length,
        types: default_gzip_types(),
    })
}

/// Update Nginx gzip configuration
#[command]
pub fn set_nginx_gzip_config(app: AppHandle, config: NginxGzipConfig) -> Result<String, String> {
    let config_dir = NginxManager::get_config_dir(&app)?;
    let nginx_conf = config_dir.join("nginx.conf");

    if !nginx_conf.exists() {
        return Err("nginx.conf not found".to_string());
    }

    let content = fs::read_to_string(&nginx_conf)
        .map_err(|e| format!("Failed to read nginx.conf: {}", e))?;

    // Remove existing gzip settings
    let lines: Vec<&str> = content.lines().collect();
    let mut filtered: Vec<String> = Vec::new();
    let mut in_http_block = false;
    let mut added_gzip = false;

    for line in lines {
        let trimmed = line.trim();

        // Skip existing gzip lines
        if trimmed.starts_with("gzip") || trimmed.starts_with("#gzip") ||
           trimmed.starts_with("# Gzip") {
            continue;
        }

        // Track http block
        if trimmed.starts_with("http") && trimmed.contains("{") {
            in_http_block = true;
        }

        // Add gzip config after http block opening
        if in_http_block && !added_gzip && trimmed.contains("default_type") {
            filtered.push(line.to_string());
            filtered.push(String::new());
            filtered.push("    # Gzip compression".to_string());
            if config.enabled {
                filtered.push("    gzip on;".to_string());
                filtered.push(format!("    gzip_comp_level {};", config.level));
                filtered.push(format!("    gzip_min_length {};", config.min_length));
                filtered.push("    gzip_vary on;".to_string());
                filtered.push("    gzip_proxied any;".to_string());
                filtered.push(format!("    gzip_types {};", config.types.join(" ")));
            } else {
                filtered.push("    gzip off;".to_string());
            }
            added_gzip = true;
            continue;
        }

        filtered.push(line.to_string());
    }

    let new_content = filtered.join("\n");
    fs::write(&nginx_conf, new_content)
        .map_err(|e| format!("Failed to write nginx.conf: {}", e))?;

    // Reload nginx if running
    if NginxManager::is_running() {
        NginxManager::reload(&app)?;
    }

    Ok("Nginx gzip configuration updated.".to_string())
}

/// Get raw nginx.conf content for editing
#[command]
pub fn get_nginx_conf_raw(app: AppHandle) -> Result<String, String> {
    let config_dir = NginxManager::get_config_dir(&app)?;
    let nginx_conf = config_dir.join("nginx.conf");

    if !nginx_conf.exists() {
        return Err("nginx.conf not found".to_string());
    }

    fs::read_to_string(&nginx_conf).map_err(|e| format!("Failed to read nginx.conf: {}", e))
}

/// Save raw nginx.conf content
#[command]
pub fn save_nginx_conf_raw(app: AppHandle, content: String) -> Result<String, String> {
    if content.trim().is_empty() {
        return Err("nginx.conf content cannot be empty".to_string());
    }

    let config_dir = NginxManager::get_config_dir(&app)?;
    let nginx_conf = config_dir.join("nginx.conf");

    if nginx_conf.exists() {
        let backup_path = nginx_conf.with_extension("conf.bak");
        fs::copy(&nginx_conf, &backup_path).ok();
    }

    fs::write(&nginx_conf, &content).map_err(|e| format!("Failed to save nginx.conf: {}", e))?;

    if NginxManager::is_running() {
        NginxManager::reload(&app)?;
    }

    Ok("nginx.conf saved successfully".to_string())
}

/// Get raw MariaDB my.ini content
#[command]
pub fn get_mariadb_conf_raw(app: AppHandle) -> Result<String, String> {
    let ini_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("data")
        .join("mariadb")
        .join("my.ini");

    if !ini_path.exists() {
        return Err("my.ini not found".to_string());
    }

    fs::read_to_string(&ini_path).map_err(|e| format!("Failed to read my.ini: {}", e))
}

/// Save raw MariaDB my.ini content
#[command]
pub fn save_mariadb_conf_raw(app: AppHandle, content: String) -> Result<String, String> {
    if content.trim().is_empty() {
        return Err("my.ini content cannot be empty".to_string());
    }

    let ini_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("data")
        .join("mariadb")
        .join("my.ini");

    if ini_path.exists() {
        let backup_path = ini_path.with_extension("ini.bak");
        fs::copy(&ini_path, &backup_path).ok();
    } else {
        if let Some(parent) = ini_path.parent() {
            fs::create_dir_all(parent).ok();
        }
    }

    fs::write(&ini_path, &content).map_err(|e| format!("Failed to save my.ini: {}", e))?;

    Ok("my.ini saved successfully".to_string())
}

/// Get raw Apache httpd.conf content
#[command]
pub fn get_apache_conf_raw(app: AppHandle) -> Result<String, String> {
    let conf_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("apache")
        .join("conf")
        .join("httpd.conf");

    if !conf_path.exists() {
        return Err("httpd.conf not found".to_string());
    }

    fs::read_to_string(&conf_path).map_err(|e| format!("Failed to read httpd.conf: {}", e))
}

/// Save raw Apache httpd.conf content
#[command]
pub fn save_apache_conf_raw(app: AppHandle, content: String) -> Result<String, String> {
    if content.trim().is_empty() {
        return Err("httpd.conf content cannot be empty".to_string());
    }

    let conf_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("apache")
        .join("conf")
        .join("httpd.conf");

    if conf_path.exists() {
        let backup_path = conf_path.with_extension("conf.bak");
        fs::copy(&conf_path, &backup_path).ok();
    } else {
        if let Some(parent) = conf_path.parent() {
            fs::create_dir_all(parent).ok();
        }
    }

    fs::write(&conf_path, &content).map_err(|e| format!("Failed to save httpd.conf: {}", e))?;

    Ok("httpd.conf saved successfully".to_string())
}

// Helper functions

fn get_opcache_status(app: &AppHandle, version: &str) -> Result<(bool, String), String> {
    let ini_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(version)
        .join("php.ini");

    if !ini_path.exists() {
        return Ok((false, "0".to_string()));
    }

    let content = fs::read_to_string(&ini_path).unwrap_or_default();

    let enabled = content.contains("opcache.enable=1") ||
                  content.contains("opcache.enable = 1");

    let memory = parse_ini_value(&content, "opcache.memory_consumption")
        .unwrap_or_else(|| "0".to_string());

    Ok((enabled, memory))
}

fn get_nginx_gzip_status(app: &AppHandle) -> Result<(bool, u8), String> {
    let config_dir = NginxManager::get_config_dir(app)?;
    let nginx_conf = config_dir.join("nginx.conf");

    if !nginx_conf.exists() {
        return Ok((false, 6));
    }

    let content = fs::read_to_string(&nginx_conf).unwrap_or_default();

    let enabled = content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("gzip") && trimmed.contains("on") && !trimmed.starts_with("#")
    });

    let level = content.lines()
        .find(|line| line.trim().starts_with("gzip_comp_level"))
        .and_then(|line| {
            line.split_whitespace()
                .nth(1)
                .and_then(|v| v.trim_end_matches(';').parse().ok())
        })
        .unwrap_or(6);

    Ok((enabled, level))
}

fn parse_ini_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn default_gzip_types() -> Vec<String> {
    vec![
        "text/plain".to_string(),
        "text/css".to_string(),
        "text/xml".to_string(),
        "text/javascript".to_string(),
        "application/json".to_string(),
        "application/javascript".to_string(),
        "application/x-javascript".to_string(),
        "application/xml".to_string(),
        "application/xml+rss".to_string(),
        "image/svg+xml".to_string(),
    ]
}

#[derive(serde::Serialize)]
pub struct CacheClearResult {
    pub opcache_cleared: bool,
    pub temp_files_cleared: u32,
    pub nginx_cache_cleared: bool,
    pub message: String,
}

/// Clear all caches (OPcache, temp files, nginx cache)
#[command]
pub fn clear_all_caches(app: AppHandle, php_version: Option<String>) -> Result<CacheClearResult, String> {
    let mut opcache_cleared = false;
    let mut temp_files_cleared = 0u32;
    let mut nginx_cache_cleared = false;
    let mut messages: Vec<String> = Vec::new();

    // 1. Clear OPcache via PHP script
    if let Some(version) = php_version {
        // Clear for specific version
        match clear_opcache(&app, &version) {
            Ok(_) => {
                opcache_cleared = true;
                messages.push(format!("OPcache cleared for PHP {}", version));
            }
            Err(e) => {
                messages.push(format!("OPcache (PHP {}): {}", version, e));
            }
        }
    } else {
        // Clear for all installed PHP versions in the bin/php directory
        let php_dir_result = app.path().app_local_data_dir().map(|dir| dir.join("bin").join("php"));
        
        if let Ok(php_dir) = php_dir_result {
            if php_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&php_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                                // Assume directory name is the version (e.g., "82")
                                if clear_opcache(&app, dir_name).is_ok() {
                                    opcache_cleared = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if opcache_cleared {
            messages.push("OPcache cleared for all PHP versions".to_string());
        }
    }

    // 2. Clear app temp/downloads directory
    match clear_temp_files(&app) {
        Ok(count) => {
            temp_files_cleared = count;
            if count > 0 {
                messages.push(format!("{} temp files cleared", count));
            }
        }
        Err(e) => {
            messages.push(format!("Temp files: {}", e));
        }
    }

    // 3. Clear Nginx proxy cache if exists
    match clear_nginx_cache(&app) {
        Ok(cleared) => {
            nginx_cache_cleared = cleared;
            if cleared {
                messages.push("Nginx cache cleared".to_string());
            }
        }
        Err(e) => {
            messages.push(format!("Nginx cache: {}", e));
        }
    }

    let message = if messages.is_empty() {
        "All caches are already clean".to_string()
    } else {
        messages.join(", ")
    };

    Ok(CacheClearResult {
        opcache_cleared,
        temp_files_cleared,
        nginx_cache_cleared,
        message,
    })
}

fn clear_opcache(app: &AppHandle, version: &str) -> Result<(), String> {
    let php_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(version)
        .join("php.exe");

    if !php_path.exists() {
        return Err("PHP not found".to_string());
    }

    // Create a temp PHP script to clear OPcache
    let temp_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("temp");

    fs::create_dir_all(&temp_dir).ok();

    let script_path = temp_dir.join("clear_opcache.php");
    let script = r#"<?php
if (function_exists('opcache_reset')) {
    opcache_reset();
    echo 'OPcache cleared';
} else {
    echo 'OPcache not available';
}
"#;

    fs::write(&script_path, script)
        .map_err(|e| format!("Failed to create script: {}", e))?;

    let mut cmd = std::process::Command::new(&php_path);
    cmd.arg(&script_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.output()
        .map_err(|e| format!("Failed to run PHP: {}", e))?;

    // Cleanup script
    let _ = fs::remove_file(&script_path);

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("cleared") {
        Ok(())
    } else if stdout.contains("not available") {
        Err("OPcache not enabled".to_string())
    } else {
        Err("OPcache reset failed".to_string())
    }
}

fn clear_temp_files(app: &AppHandle) -> Result<u32, String> {
    let downloads_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("downloads");

    let temp_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("temp");

    let mut count = 0u32;

    // Clear downloads directory
    if downloads_dir.exists() {
        if let Ok(entries) = fs::read_dir(&downloads_dir) {
            for entry in entries.flatten() {
                if fs::remove_file(entry.path()).is_ok() {
                    count += 1;
                }
            }
        }
    }

    // Clear temp directory
    if temp_dir.exists() {
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if fs::remove_file(&path).is_ok() {
                        count += 1;
                    }
                } else if path.is_dir() {
                    if fs::remove_dir_all(&path).is_ok() {
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

fn clear_nginx_cache(app: &AppHandle) -> Result<bool, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("nginx")
        .join("cache");

    if !cache_dir.exists() {
        return Ok(false);
    }

    // Clear cache directory contents
    if let Ok(entries) = fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let _ = fs::remove_dir_all(&path);
            } else {
                let _ = fs::remove_file(&path);
            }
        }
    }

    Ok(true)
}
