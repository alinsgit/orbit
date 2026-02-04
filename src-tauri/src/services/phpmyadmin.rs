use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write as IoWrite};
use std::path::PathBuf;
use zip::ZipArchive;

const PHPMYADMIN_VERSION: &str = "5.2.2";
const PHPMYADMIN_DOWNLOAD_URL: &str = "https://files.phpmyadmin.net/phpMyAdmin/5.2.2/phpMyAdmin-5.2.2-all-languages.zip";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpMyAdminStatus {
    pub installed: bool,
    pub path: String,
    pub url: String,
    pub version: String,
}

pub struct PhpMyAdminManager;

impl PhpMyAdminManager {
    /// Get phpmyadmin directory
    pub fn get_phpmyadmin_dir(bin_path: &PathBuf) -> PathBuf {
        bin_path.join("phpmyadmin")
    }

    /// Get phpmyadmin index file path
    pub fn get_phpmyadmin_path(bin_path: &PathBuf) -> PathBuf {
        Self::get_phpmyadmin_dir(bin_path).join("index.php")
    }

    /// Check if phpmyadmin is installed
    pub fn is_installed(bin_path: &PathBuf) -> bool {
        Self::get_phpmyadmin_path(bin_path).exists()
    }

    /// Download and install PhpMyAdmin
    pub async fn install(bin_path: &PathBuf) -> Result<String, String> {
        let phpmyadmin_dir = Self::get_phpmyadmin_dir(bin_path);

        // Remove existing installation if present
        if phpmyadmin_dir.exists() {
            fs::remove_dir_all(&phpmyadmin_dir)
                .map_err(|e| format!("Failed to remove existing phpmyadmin: {}", e))?;
        }

        // Create temp directory for download
        let temp_dir = bin_path.join("temp");
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)
                .map_err(|e| format!("Failed to create temp dir: {}", e))?;
        }

        let zip_path = temp_dir.join("phpmyadmin.zip");

        // Download PhpMyAdmin
        let response = reqwest::get(PHPMYADMIN_DOWNLOAD_URL)
            .await
            .map_err(|e| format!("Failed to download PhpMyAdmin: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download PhpMyAdmin: HTTP {}",
                response.status()
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read PhpMyAdmin: {}", e))?;

        fs::write(&zip_path, &bytes).map_err(|e| format!("Failed to save PhpMyAdmin: {}", e))?;

        // Extract zip file
        let file =
            fs::File::open(&zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;

        let mut archive =
            ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

        // Create phpmyadmin directory
        fs::create_dir_all(&phpmyadmin_dir)
            .map_err(|e| format!("Failed to create phpmyadmin dir: {}", e))?;

        // Extract files - PhpMyAdmin zip has a root folder like "phpMyAdmin-5.2.1-all-languages"
        let root_folder = format!("phpMyAdmin-{}-all-languages/", PHPMYADMIN_VERSION);

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;

            let name = file.name().to_string();

            // Skip if not in our root folder
            if !name.starts_with(&root_folder) {
                continue;
            }

            // Get relative path by removing root folder prefix
            let relative_path = name.strip_prefix(&root_folder).unwrap_or(&name);

            if relative_path.is_empty() {
                continue;
            }

            let outpath = phpmyadmin_dir.join(relative_path);

            if name.ends_with('/') {
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            } else {
                if let Some(parent) = outpath.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                    }
                }

                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| format!("Failed to create file: {}", e))?;

                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)
                    .map_err(|e| format!("Failed to read file content: {}", e))?;

                outfile
                    .write_all(&buffer)
                    .map_err(|e| format!("Failed to write file: {}", e))?;
            }
        }

        // Clean up zip file
        let _ = fs::remove_file(&zip_path);

        // Create .user.ini for PHP 8.4 compatibility (loaded before any PHP code)
        Self::create_user_ini(&phpmyadmin_dir)?;

        // Create config file
        Self::create_config(&phpmyadmin_dir)?;

        // Create wrapper to handle session/output buffering issues
        Self::create_wrapper(&phpmyadmin_dir)?;

        Ok(format!(
            "PhpMyAdmin {} installed successfully",
            PHPMYADMIN_VERSION
        ))
    }

    /// Create .user.ini file for PHP 8.4 compatibility
    /// This file is loaded by PHP automatically and applies settings before any code runs
    fn create_user_ini(phpmyadmin_dir: &PathBuf) -> Result<(), String> {
        let user_ini_path = phpmyadmin_dir.join(".user.ini");

        let user_ini_content = r#"; PHP settings for PhpMyAdmin
; This file is automatically loaded by PHP before any script execution
; Critical for PHP 8.4 compatibility to suppress deprecation warnings

display_errors = Off
display_startup_errors = Off
error_reporting = E_ALL & ~E_NOTICE & ~E_STRICT & ~E_DEPRECATED & ~E_WARNING
log_errors = On

; Session settings
session.cookie_httponly = 1
session.use_strict_mode = 1
session.use_only_cookies = 1

; Memory and execution limits for large databases
memory_limit = 512M
max_execution_time = 600
upload_max_filesize = 128M
post_max_size = 128M
"#;

        fs::write(&user_ini_path, user_ini_content)
            .map_err(|e| format!("Failed to create .user.ini: {}", e))?;

        Ok(())
    }

    /// Create a wrapper index file to handle PHP session issues
    fn create_wrapper(phpmyadmin_dir: &PathBuf) -> Result<(), String> {
        let index_path = phpmyadmin_dir.join("index.php");
        let original_path = phpmyadmin_dir.join("index_original.php");

        // Rename original index.php if it exists and wrapper doesn't exist yet
        if index_path.exists() && !original_path.exists() {
            fs::rename(&index_path, &original_path)
                .map_err(|e| format!("Failed to rename index.php: {}", e))?;
        }

        // Create wrapper - error suppression MUST happen before anything else
        // Using @ operator and early ini_set to catch PHP 8.4 deprecation warnings
        let wrapper_content = r#"<?php
/**
 * PhpMyAdmin Wrapper
 * Generated by Orbit Local Server
 * Handles session and output buffering issues for PHP 8.4+
 */

// CRITICAL: Suppress ALL errors before any code runs
// This must be at the very top to catch autoloader deprecation warnings
@ini_set('display_errors', '0');
@ini_set('display_startup_errors', '0');
@error_reporting(0);

// Start output buffering immediately to catch any stray output
ob_start();

// Now set proper error reporting (without deprecated/warnings)
error_reporting(E_ALL & ~E_NOTICE & ~E_STRICT & ~E_DEPRECATED & ~E_WARNING);

// Session configuration - must be before session_start
@ini_set('session.cookie_httponly', '1');
@ini_set('session.use_strict_mode', '1');
@ini_set('session.use_only_cookies', '1');

// Include the original phpMyAdmin index with error suppression on include
@require_once __DIR__ . '/index_original.php';
"#;

        fs::write(&index_path, wrapper_content)
            .map_err(|e| format!("Failed to create wrapper: {}", e))?;

        Ok(())
    }

    /// Create PhpMyAdmin configuration
    fn create_config(phpmyadmin_dir: &PathBuf) -> Result<(), String> {
        let config_path = phpmyadmin_dir.join("config.inc.php");

        // Generate a random blowfish secret
        let blowfish_secret: String = (0..32)
            .map(|_| {
                let idx = rand::random::<usize>() % 62;
                let chars: &[u8] =
                    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                chars[idx] as char
            })
            .collect();

        let config_content = format!(
            r#"<?php
/**
 * phpMyAdmin Configuration
 * Generated by Orbit Local Server
 */

// CRITICAL: Suppress errors at the very start for PHP 8.4 compatibility
@ini_set('display_errors', '0');
@ini_set('display_startup_errors', '0');
@error_reporting(0);

// Start output buffering to prevent "headers already sent" errors
ob_start();

// Error reporting - hide all notices, warnings, and deprecation
error_reporting(E_ALL & ~E_NOTICE & ~E_STRICT & ~E_DEPRECATED & ~E_WARNING);

// Blowfish secret for cookie auth
$cfg['blowfish_secret'] = '{}';

// Server configuration
$i = 0;
$i++;

$cfg['Servers'][$i]['auth_type'] = 'cookie';
$cfg['Servers'][$i]['host'] = '127.0.0.1';
$cfg['Servers'][$i]['port'] = '3306';
$cfg['Servers'][$i]['compress'] = false;
$cfg['Servers'][$i]['AllowNoPassword'] = true;

// Directories for temp files
$cfg['UploadDir'] = '';
$cfg['SaveDir'] = '';

// Theme and appearance
$cfg['ThemeDefault'] = 'pmahomme';

// Enable export/import
$cfg['Export']['compression'] = 'gzip';
$cfg['Import']['charset'] = 'utf-8';

// Increase session timeout
$cfg['LoginCookieValidity'] = 28800;

// Allow large uploads
$cfg['ExecTimeLimit'] = 600;
$cfg['MemoryLimit'] = '512M';

// Allow embedding in iframe (for Orbit integration)
$cfg['AllowThirdPartyFraming'] = true;
"#,
            blowfish_secret
        );

        fs::write(&config_path, config_content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }

    /// Uninstall PhpMyAdmin
    pub fn uninstall(bin_path: &PathBuf) -> Result<(), String> {
        let phpmyadmin_dir = Self::get_phpmyadmin_dir(bin_path);
        if phpmyadmin_dir.exists() {
            fs::remove_dir_all(&phpmyadmin_dir)
                .map_err(|e| format!("Failed to remove phpmyadmin: {}", e))?;
        }
        Ok(())
    }

    /// Get status
    pub fn get_status(bin_path: &PathBuf) -> PhpMyAdminStatus {
        let installed = Self::is_installed(bin_path);
        let phpmyadmin_path = Self::get_phpmyadmin_path(bin_path)
            .to_string_lossy()
            .to_string();

        // PhpMyAdmin is served through nginx on port 8081
        let phpmyadmin_url = if installed {
            "http://localhost:8081/".to_string()
        } else {
            String::new()
        };

        PhpMyAdminStatus {
            installed,
            path: phpmyadmin_path,
            url: phpmyadmin_url,
            version: PHPMYADMIN_VERSION.to_string(),
        }
    }

    /// Create nginx config for phpmyadmin
    pub fn create_nginx_config(bin_path: &PathBuf, php_port: u16) -> Result<String, String> {
        let phpmyadmin_dir = Self::get_phpmyadmin_dir(bin_path);

        if !phpmyadmin_dir.exists() {
            return Err("PhpMyAdmin is not installed".to_string());
        }

        let config = format!(
            r#"# PhpMyAdmin Database Manager
server {{
    listen       8081;
    server_name  localhost;
    root         "{}";

    index index.php index.html;

    # Client body size for large imports
    client_max_body_size 128M;

    location / {{
        try_files $uri $uri/ /index.php?$query_string;
    }}

    location ~ \.php$ {{
        fastcgi_pass   127.0.0.1:{};
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        include        fastcgi_params;

        # Buffer settings for large responses
        fastcgi_buffer_size 128k;
        fastcgi_buffers 4 256k;
        fastcgi_busy_buffers_size 256k;

        # Timeouts for long-running queries
        fastcgi_read_timeout 600;
        fastcgi_send_timeout 600;

        # PHP value overrides for error suppression
        fastcgi_param PHP_VALUE "display_errors=Off
display_startup_errors=Off
error_reporting=E_ALL & ~E_NOTICE & ~E_STRICT & ~E_DEPRECATED & ~E_WARNING";

        # Override X-Frame-Options from PHP (allow iframe embedding)
        fastcgi_hide_header X-Frame-Options;
    }}

    # Deny access to sensitive files
    location ~ /\.(ht|git|svn|user\.ini) {{
        deny all;
    }}
}}
"#,
            phpmyadmin_dir.to_string_lossy().replace("\\", "/"),
            php_port
        );

        // Write config to nginx sites-enabled
        let nginx_conf_dir = bin_path.join("nginx").join("conf").join("sites-enabled");

        if !nginx_conf_dir.exists() {
            fs::create_dir_all(&nginx_conf_dir)
                .map_err(|e| format!("Failed to create nginx config dir: {}", e))?;
        }

        let config_path = nginx_conf_dir.join("phpmyadmin.conf");

        fs::write(&config_path, &config)
            .map_err(|e| format!("Failed to write nginx config: {}", e))?;

        Ok(config_path.to_string_lossy().to_string())
    }

    /// Remove nginx config for phpmyadmin
    pub fn remove_nginx_config(bin_path: &PathBuf) -> Result<(), String> {
        let nginx_conf_dir = bin_path.join("nginx").join("conf").join("sites-enabled");
        let config_path = nginx_conf_dir.join("phpmyadmin.conf");

        if config_path.exists() {
            fs::remove_file(&config_path)
                .map_err(|e| format!("Failed to remove nginx config: {}", e))?;
        }

        Ok(())
    }
}
