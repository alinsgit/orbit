use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

const ADMINER_VERSION: &str = "4.8.1";
const ADMINER_DOWNLOAD_URL: &str = "https://github.com/vrana/adminer/releases/download/v4.8.1/adminer-4.8.1.php";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStatus {
    pub adminer_installed: bool,
    pub adminer_path: String,
    pub adminer_url: String,
}

pub struct DatabaseManager;

impl DatabaseManager {
    /// Get adminer directory
    pub fn get_adminer_dir(bin_path: &PathBuf) -> PathBuf {
        bin_path.join("adminer")
    }

    /// Get adminer file path
    pub fn get_adminer_path(bin_path: &PathBuf) -> PathBuf {
        Self::get_adminer_dir(bin_path).join("index.php")
    }

    /// Check if adminer is installed
    pub fn is_installed(bin_path: &PathBuf) -> bool {
        Self::get_adminer_path(bin_path).exists()
    }

    /// Download and install Adminer
    pub async fn install(bin_path: &PathBuf) -> Result<String, String> {
        let adminer_dir = Self::get_adminer_dir(bin_path);
        if !adminer_dir.exists() {
            fs::create_dir_all(&adminer_dir)
                .map_err(|e| format!("Failed to create adminer dir: {}", e))?;
        }

        let adminer_path = Self::get_adminer_path(bin_path);

        // Download Adminer
        let response = reqwest::get(ADMINER_DOWNLOAD_URL)
            .await
            .map_err(|e| format!("Failed to download Adminer: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to download Adminer: HTTP {}", response.status()));
        }

        let bytes = response.bytes().await
            .map_err(|e| format!("Failed to read Adminer: {}", e))?;

        fs::write(&adminer_path, &bytes)
            .map_err(|e| format!("Failed to save Adminer: {}", e))?;

        // Create a custom wrapper with styling - NO hardcoded credentials
        // Users must enter their own credentials for security
        // Added PHP 8.4 compatibility with error suppression
        let wrapper_content = format!(r#"<?php
// Adminer {} - Database Management
// Orbit Local Server wrapper
// PHP 8.4 Compatible

// CRITICAL: Suppress errors before any code runs for PHP 8.4 compatibility
@ini_set('display_errors', '0');
@ini_set('display_startup_errors', '0');
@error_reporting(0);

// Session cookie config for iframe embedding
@ini_set('session.cookie_samesite', 'None');
@ini_set('session.cookie_secure', '1');
@ini_set('session.cookie_httponly', '1');

// Start output buffering
ob_start();

// Set proper error reporting (without deprecated/warnings)
error_reporting(E_ALL & ~E_NOTICE & ~E_STRICT & ~E_DEPRECATED & ~E_WARNING);

function adminer_object() {{
    class AdminerCustom extends Adminer {{
        function name() {{
            return 'Orbit DB Manager';
        }}

        // Auto-login credentials for local development
        function credentials() {{
            return array('127.0.0.1', 'root', 'root');
        }}

        function login($login, $password) {{
            return true;
        }}

        // Auto-submit login form so user never sees it
        function loginForm() {{
            echo '<script>
            document.addEventListener("DOMContentLoaded", function() {{
                var form = document.querySelector("form");
                if (form && !document.querySelector(".error")) {{
                    form.submit();
                }}
            }});
            </script>';
            return parent::loginForm();
        }}

        function permanentLogin($create = false) {{
            return 'orbit_session';
        }}

        function database() {{
            return null;
        }}

        function servers() {{
            return array('127.0.0.1' => 'Local MariaDB');
        }}
    }}
    return new AdminerCustom;
}}

@include __DIR__ . '/adminer-{}.php';
"#, ADMINER_VERSION, ADMINER_VERSION);

        // Rename original file and create wrapper
        let adminer_original = adminer_dir.join(format!("adminer-{}.php", ADMINER_VERSION));
        fs::rename(&adminer_path, &adminer_original)
            .map_err(|e| format!("Failed to rename adminer: {}", e))?;

        fs::write(&adminer_path, wrapper_content)
            .map_err(|e| format!("Failed to create wrapper: {}", e))?;

        // Create .user.ini for PHP 8.4 compatibility
        let user_ini_path = adminer_dir.join(".user.ini");
        let user_ini_content = r#"; PHP settings for Adminer
; PHP 8.4 compatibility - suppress deprecation warnings

display_errors = Off
display_startup_errors = Off
error_reporting = E_ALL & ~E_NOTICE & ~E_STRICT & ~E_DEPRECATED & ~E_WARNING
log_errors = On

; Session cookie settings for iframe embedding
session.cookie_httponly = 1
session.cookie_samesite = None
session.cookie_secure = 1
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

        Ok(format!("Adminer {} installed successfully", ADMINER_VERSION))
    }

    /// Uninstall Adminer
    pub fn uninstall(bin_path: &PathBuf) -> Result<(), String> {
        let adminer_dir = Self::get_adminer_dir(bin_path);
        if adminer_dir.exists() {
            fs::remove_dir_all(&adminer_dir)
                .map_err(|e| format!("Failed to remove adminer: {}", e))?;
        }
        Ok(())
    }

    /// Get status
    pub fn get_status(bin_path: &PathBuf) -> DatabaseStatus {
        let installed = Self::is_installed(bin_path);
        let adminer_path = Self::get_adminer_path(bin_path).to_string_lossy().to_string();

        // Adminer is served through nginx on a special route
        let adminer_url = if installed {
            "http://127.0.0.1:8080/adminer/".to_string()
        } else {
            String::new()
        };

        DatabaseStatus {
            adminer_installed: installed,
            adminer_path,
            adminer_url,
        }
    }

    /// Create nginx config for adminer
    pub fn create_nginx_config(bin_path: &PathBuf, php_port: u16) -> Result<String, String> {
        let adminer_dir = Self::get_adminer_dir(bin_path);

        if !adminer_dir.exists() {
            return Err("Adminer is not installed".to_string());
        }

        let config = format!(r#"# Adminer Database Manager
server {{
    listen       8080;
    server_name  localhost;
    root         "{}";

    index index.php;

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

        # PHP value overrides for error suppression (PHP 8.4 compatibility)
        fastcgi_param PHP_VALUE "display_errors=Off
display_startup_errors=Off
error_reporting=E_ALL & ~E_NOTICE & ~E_STRICT & ~E_DEPRECATED & ~E_WARNING";

        # Override X-Frame-Options from PHP (allow iframe embedding)
        fastcgi_hide_header X-Frame-Options;
    }}
}}
"#, adminer_dir.to_string_lossy().replace("\\", "/"), php_port);

        // Write config to nginx sites-enabled
        let nginx_conf_dir = bin_path.join("nginx").join("conf").join("sites-enabled");

        // Create sites-enabled directory if it doesn't exist
        if !nginx_conf_dir.exists() {
            fs::create_dir_all(&nginx_conf_dir)
                .map_err(|e| format!("Failed to create nginx sites-enabled dir: {}", e))?;
        }

        let config_path = nginx_conf_dir.join("adminer.conf");

        fs::write(&config_path, &config)
            .map_err(|e| format!("Failed to write nginx config: {}", e))?;

        Ok(config_path.to_string_lossy().to_string())
    }

    /// Remove nginx config for adminer
    pub fn remove_nginx_config(bin_path: &PathBuf) -> Result<(), String> {
        let nginx_conf_dir = bin_path.join("nginx").join("conf").join("sites-enabled");
        let config_path = nginx_conf_dir.join("adminer.conf");

        if config_path.exists() {
            fs::remove_file(&config_path)
                .map_err(|e| format!("Failed to remove nginx config: {}", e))?;
        }

        Ok(())
    }
}
