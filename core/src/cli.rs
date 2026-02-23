//! Orbit CLI — Command-line interface for managing local development services
//!
//! Usage:
//!   orbit-cli status              Show status of all services
//!   orbit-cli start <service>     Start a service
//!   orbit-cli start --all         Start all installed services
//!   orbit-cli stop <service>      Stop a service
//!   orbit-cli stop --all          Stop all services
//!   orbit-cli restart <service>   Restart a service
//!   orbit-cli restart --all       Restart all services
//!   orbit-cli list                List available services to install
//!   orbit-cli sites [--json]      List configured sites
//!   orbit-cli info                Show environment info
//!   orbit-cli logs list           List log files
//!   orbit-cli logs show <name>    Show log contents
//!   orbit-cli logs clear <name>   Clear a log file
//!   orbit-cli db list             List databases
//!   orbit-cli db create <name>    Create a database
//!   orbit-cli db drop <name>      Drop a database
//!   orbit-cli db export <name>    Export a database
//!   orbit-cli db import <name>    Import a SQL file
//!   orbit-cli open <target>       Open a site/tool in browser
//!   orbit-cli php list            List PHP versions
//!   orbit-cli php ext <version>   Manage PHP extensions
//!   orbit-cli hosts list|add|remove  Manage hosts file
//!   orbit-cli composer <args>     Run composer via Orbit's PHP
//!   orbit-cli install <service>   Install a service (nginx, php, redis, etc.)
//!   orbit-cli uninstall <service> Uninstall a service

use clap::{Parser, Subcommand};
use colored::*;
use serde::Deserialize;
use std::fs;
use std::io::{BufRead, BufReader, Read as IoRead, Seek, SeekFrom, Write as IoWrite};
use std::path::PathBuf;
use std::process::Command;

// ─── Path Resolution ──────────────────────────────────────────────

/// Get the Orbit data directory (matches Tauri's app_local_data_dir).
fn get_orbit_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
            format!("{}\\AppData\\Local", home)
        });
        PathBuf::from(local_app_data).join("com.orbit.dev")
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
        PathBuf::from(home).join("Library/Application Support/com.orbit.dev")
    }
    #[cfg(target_os = "linux")]
    {
        let data_home = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/default".to_string());
            format!("{}/.local/share", home)
        });
        PathBuf::from(data_home).join("com.orbit.dev")
    }
}

fn get_bin_dir() -> PathBuf {
    get_orbit_data_dir().join("bin")
}

fn get_config_dir() -> PathBuf {
    get_orbit_data_dir().join("config")
}

// ─── Site Store Types (CLI-only, Tauri-free) ─────────────────────

#[derive(Deserialize)]
struct CliSiteStore {
    #[allow(dead_code)]
    version: String,
    sites: Vec<CliSiteMetadata>,
}

#[derive(Deserialize, Clone)]
struct CliSiteMetadata {
    domain: String,
    path: String,
    port: u16,
    php_version: Option<String>,
    #[serde(default)]
    ssl_enabled: bool,
    #[serde(default = "default_web_server")]
    web_server: String,
    #[serde(default)]
    created_at: String,
}

fn default_web_server() -> String {
    "nginx".to_string()
}

fn read_sites_store() -> Result<CliSiteStore, String> {
    let store_path = get_config_dir().join("sites.json");
    if !store_path.exists() {
        return Ok(CliSiteStore {
            version: "1".to_string(),
            sites: vec![],
        });
    }
    let content = fs::read_to_string(&store_path)
        .map_err(|e| format!("Failed to read sites.json: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse sites.json: {}", e))
}

// ─── Service Discovery ────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ServiceInfo {
    name: String,
    version: String,
    path: String,
    service_type: String,
}

fn hidden_command(program: &PathBuf) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn parse_version_output(exe_path: &PathBuf, args: &[&str], pattern: &str, offset: usize) -> String {
    let output = hidden_command(exe_path)
        .args(args)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let combined = format!("{}{}", stdout, stderr);

            if let Some(pos) = combined.find(pattern) {
                let version_start = pos + offset;
                let version_end = combined[version_start..]
                    .find(|c: char| !c.is_ascii_digit() && c != '.')
                    .map(|i| version_start + i)
                    .unwrap_or(combined.len());
                let version = &combined[version_start..version_end];
                if !version.is_empty() {
                    return version.to_string();
                }
            }
            "unknown".to_string()
        }
        Err(_) => "unknown".to_string(),
    }
}

fn scan_services(bin_path: &PathBuf) -> Vec<ServiceInfo> {
    let mut services = Vec::new();

    if !bin_path.exists() {
        return services;
    }

    // Nginx
    let nginx_exe = bin_path.join("nginx").join("nginx.exe");
    if nginx_exe.exists() {
        let version = parse_version_output(&nginx_exe, &["-v"], "nginx/", 6);
        services.push(ServiceInfo {
            name: "nginx".to_string(),
            version,
            path: nginx_exe.to_string_lossy().to_string(),
            service_type: "nginx".to_string(),
        });
    }

    // MariaDB
    let mariadb_paths = [
        bin_path.join("mariadb").join("mariadbd.exe"),
        bin_path.join("mariadb").join("bin").join("mariadbd.exe"),
        bin_path.join("mariadb").join("mysqld.exe"),
        bin_path.join("mariadb").join("bin").join("mysqld.exe"),
    ];
    for exe_path in &mariadb_paths {
        if exe_path.exists() {
            let version = parse_version_output(exe_path, &["--version"], "Ver ", 4);
            services.push(ServiceInfo {
                name: "mariadb".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "mariadb".to_string(),
            });
            break;
        }
    }

    // PHP versions
    let php_root = bin_path.join("php");
    if php_root.exists() {
        if let Ok(entries) = std::fs::read_dir(&php_root) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let version_dir = entry.file_name();
                        let version_str = version_dir.to_string_lossy().to_string();
                        let exe_path = entry.path().join("php-cgi.exe");
                        if exe_path.exists() {
                            let version = parse_version_output(&exe_path, &["-v"], "PHP ", 4);
                            services.push(ServiceInfo {
                                name: format!("php-{}", version_str),
                                version,
                                path: exe_path.to_string_lossy().to_string(),
                                service_type: "php".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Redis
    let redis_exe = bin_path.join("redis").join("redis-server.exe");
    if redis_exe.exists() {
        let version = parse_version_output(&redis_exe, &["--version"], "v=", 2);
        services.push(ServiceInfo {
            name: "redis".to_string(),
            version,
            path: redis_exe.to_string_lossy().to_string(),
            service_type: "redis".to_string(),
        });
    }

    // Apache
    let apache_paths = [
        bin_path.join("apache").join("bin").join("httpd.exe"),
        bin_path.join("apache").join("httpd.exe"),
    ];
    for exe_path in &apache_paths {
        if exe_path.exists() {
            let version = parse_version_output(exe_path, &["-v"], "Apache/", 7);
            services.push(ServiceInfo {
                name: "apache".to_string(),
                version,
                path: exe_path.to_string_lossy().to_string(),
                service_type: "apache".to_string(),
            });
            break;
        }
    }

    // Node.js
    let node_exe = bin_path.join("nodejs").join("node.exe");
    if node_exe.exists() {
        let output = hidden_command(&node_exe)
            .arg("--version")
            .output();
        let version = match output {
            Ok(out) => {
                let v = String::from_utf8_lossy(&out.stdout).trim().trim_start_matches('v').to_string();
                if v.is_empty() { "unknown".to_string() } else { v }
            }
            Err(_) => "unknown".to_string(),
        };
        services.push(ServiceInfo {
            name: "nodejs".to_string(),
            version,
            path: node_exe.to_string_lossy().to_string(),
            service_type: "nodejs".to_string(),
        });
    }

    // Mailpit
    let mailpit_exe = bin_path.join("mailpit").join("mailpit.exe");
    if mailpit_exe.exists() {
        services.push(ServiceInfo {
            name: "mailpit".to_string(),
            version: "installed".to_string(),
            path: mailpit_exe.to_string_lossy().to_string(),
            service_type: "mailpit".to_string(),
        });
    }

    // Composer
    let composer_phar = bin_path.join("composer").join("composer.phar");
    if composer_phar.exists() {
        services.push(ServiceInfo {
            name: "composer".to_string(),
            version: "installed".to_string(),
            path: composer_phar.to_string_lossy().to_string(),
            service_type: "composer".to_string(),
        });
    }

    // PostgreSQL (check both flattened and nested structures)
    let pg_paths = [
        bin_path.join("postgresql").join("bin").join("postgres.exe"),
        bin_path.join("postgresql").join("pgsql").join("bin").join("postgres.exe"),
    ];
    for pg_exe in &pg_paths {
        if pg_exe.exists() {
            let version = parse_version_output(pg_exe, &["--version"], "postgres (PostgreSQL) ", 22);
            services.push(ServiceInfo {
                name: "postgresql".to_string(),
                version,
                path: pg_exe.to_string_lossy().to_string(),
                service_type: "postgresql".to_string(),
            });
            break;
        }
    }

    // MongoDB
    let mongo_exe = bin_path.join("mongodb").join("bin").join("mongod.exe");
    if mongo_exe.exists() {
        services.push(ServiceInfo {
            name: "mongodb".to_string(),
            version: "installed".to_string(),
            path: mongo_exe.to_string_lossy().to_string(),
            service_type: "mongodb".to_string(),
        });
    }

    // Go
    let go_paths = [
        bin_path.join("go").join("bin").join("go.exe"),
        bin_path.join("go").join("go.exe"),
    ];
    for go_exe in &go_paths {
        if go_exe.exists() {
            let version = parse_version_output(go_exe, &["version"], "go", 2);
            services.push(ServiceInfo {
                name: "go".to_string(),
                version,
                path: go_exe.to_string_lossy().to_string(),
                service_type: "go".to_string(),
            });
            break;
        }
    }

    // Deno
    let deno_exe = bin_path.join("deno").join("deno.exe");
    if deno_exe.exists() {
        let version = parse_version_output(&deno_exe, &["--version"], "deno ", 5);
        services.push(ServiceInfo {
            name: "deno".to_string(),
            version,
            path: deno_exe.to_string_lossy().to_string(),
            service_type: "deno".to_string(),
        });
    }

    // Bun
    let bun_exe = bin_path.join("bun").join("bun.exe");
    if bun_exe.exists() {
        let version = parse_version_output(&bun_exe, &["--version"], "", 0);
        services.push(ServiceInfo {
            name: "bun".to_string(),
            version,
            path: bun_exe.to_string_lossy().to_string(),
            service_type: "bun".to_string(),
        });
    }

    // Python
    let python_exe = bin_path.join("python").join("python.exe");
    if python_exe.exists() {
        let version = parse_version_output(&python_exe, &["--version"], "Python ", 7);
        services.push(ServiceInfo {
            name: "python".to_string(),
            version,
            path: python_exe.to_string_lossy().to_string(),
            service_type: "python".to_string(),
        });
    }

    // Rust
    let rust_paths = [
        bin_path.join("rust").join("rustup-init.exe"),
        bin_path.join("misc").join("rust").join("rustup-init.exe"),
    ];
    for rust_exe in &rust_paths {
        if rust_exe.exists() {
            services.push(ServiceInfo {
                name: "rust".to_string(),
                version: "installed".to_string(),
                path: rust_exe.to_string_lossy().to_string(),
                service_type: "rust".to_string(),
            });
            break;
        }
    }

    services
}

// ─── Process Management ───────────────────────────────────────────

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
        || std::net::TcpListener::bind(format!("0.0.0.0:{}", port)).is_err()
}

fn get_service_port(name: &str) -> Option<u16> {
    if name.contains("nginx") {
        Some(80)
    } else if name.contains("apache") {
        Some(80)
    } else if name.contains("mariadb") {
        Some(3306)
    } else if name.contains("redis") {
        Some(6379)
    } else if name.contains("php") {
        // php-8.4 → minor=4 → 9000+4=9004 (matches GUI logic)
        let version_str = name.strip_prefix("php-").unwrap_or("8.4");
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() >= 2 {
            let minor: u16 = parts[1].parse().unwrap_or(4);
            Some(9000 + minor)
        } else {
            Some(9004)
        }
    } else if name.contains("mailpit") {
        Some(8025)
    } else if name.contains("postgresql") {
        Some(5432)
    } else if name.contains("mongodb") {
        Some(27017)
    } else {
        None
    }
}

fn get_process_image_names(name: &str) -> Vec<&'static str> {
    if name.contains("nginx") {
        vec!["nginx.exe"]
    } else if name.contains("php") {
        vec!["php-cgi.exe"]
    } else if name.contains("mariadb") {
        vec!["mariadbd.exe", "mysqld.exe"]
    } else if name.contains("redis") {
        vec!["redis-server.exe"]
    } else if name.contains("apache") {
        vec!["httpd.exe"]
    } else if name.contains("mailpit") {
        vec!["mailpit.exe"]
    } else if name.contains("postgresql") {
        vec!["postgres.exe"]
    } else if name.contains("mongodb") {
        vec!["mongod.exe"]
    } else {
        vec![]
    }
}

fn is_service_running(name: &str) -> bool {
    if let Some(port) = get_service_port(name) {
        is_port_in_use(port)
    } else {
        false
    }
}

fn start_service_process(service: &ServiceInfo) -> Result<u32, String> {
    let exe_path = PathBuf::from(&service.path);
    let bin_dir = get_bin_dir();

    let (exe, args) = match service.service_type.as_str() {
        "nginx" => {
            let nginx_dir = bin_dir.join("nginx");
            let exe = nginx_dir.join("nginx.exe");
            (exe, vec![])
        }
        "php" => {
            let port = get_service_port(&service.name).unwrap_or(9084);
            (exe_path.clone(), vec!["-b".to_string(), format!("127.0.0.1:{}", port)])
        }
        "mariadb" => {
            let data_dir = bin_dir.join("data").join("mariadb");
            let config_path = data_dir.join("my.ini");
            let mut args = Vec::new();
            if config_path.exists() {
                args.push(format!("--defaults-file={}", config_path.display()));
            }
            args.push("--console".to_string());
            args.push(format!("--datadir={}", data_dir.display()));
            (exe_path.clone(), args)
        }
        "redis" => {
            // Pass config as relative path — Cygwin-based Redis misinterprets
            // absolute Windows paths by prepending /cygdrive/...
            let mut args = Vec::new();
            if let Some(parent) = exe_path.parent() {
                let config = parent.join("redis.conf");
                if config.exists() {
                    args.push("redis.conf".to_string());
                }
            }
            (exe_path.clone(), args)
        }
        "apache" => {
            (exe_path.clone(), vec![])
        }
        "mailpit" => {
            (exe_path.clone(), vec![])
        }
        "postgresql" => {
            let data_dir = bin_dir.join("data").join("postgres");
            (exe_path.clone(), vec!["-D".to_string(), data_dir.display().to_string()])
        }
        "mongodb" => {
            let data_dir = bin_dir.join("data").join("mongodb");
            fs::create_dir_all(&data_dir).ok();
            (exe_path.clone(), vec![
                "--dbpath".to_string(), data_dir.display().to_string(),
                "--port".to_string(), "27017".to_string(),
            ])
        }
        _ => {
            return Err(format!("Unknown service type: {}", service.service_type));
        }
    };

    if !exe.exists() {
        return Err(format!("Executable not found: {}", exe.display()));
    }

    let mut cmd = hidden_command(&exe);
    for arg in &args {
        cmd.arg(arg);
    }

    match cmd.spawn() {
        Ok(child) => Ok(child.id()),
        Err(e) => Err(format!("Failed to start {}: {}", service.name, e)),
    }
}

fn stop_service_process(name: &str) -> Result<(), String> {
    let image_names = get_process_image_names(name);

    if image_names.is_empty() {
        return Err(format!("Don't know how to stop: {}", name));
    }

    let mut any_killed = false;
    for process_name in &image_names {
        let mut cmd = Command::new("taskkill");
        cmd.args(["/F", "/IM", process_name]);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                any_killed = true;
            }
        }
    }

    if any_killed {
        Ok(())
    } else {
        if let Some(port) = get_service_port(name) {
            if !is_port_in_use(port) {
                return Ok(());
            }
        }
        Err(format!("Could not stop {}", name))
    }
}

// ─── Helper: MariaDB client discovery ─────────────────────────────

fn find_mariadb_client(bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let mariadb_root = bin_dir.join("mariadb");
    let paths = [
        mariadb_root.join("mariadb.exe"),
        mariadb_root.join("mysql.exe"),
        mariadb_root.join("bin").join("mariadb.exe"),
        mariadb_root.join("bin").join("mysql.exe"),
    ];
    for path in paths {
        if path.exists() {
            return Ok(path);
        }
    }
    Err("MariaDB client not found (mysql.exe / mariadb.exe)".to_string())
}

fn find_mariadb_dump(bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let mariadb_root = bin_dir.join("mariadb");
    let paths = [
        mariadb_root.join("mariadb-dump.exe"),
        mariadb_root.join("mysqldump.exe"),
        mariadb_root.join("bin").join("mariadb-dump.exe"),
        mariadb_root.join("bin").join("mysqldump.exe"),
    ];
    for path in paths {
        if path.exists() {
            return Ok(path);
        }
    }
    Err("MariaDB dump not found (mysqldump.exe / mariadb-dump.exe)".to_string())
}

// ─── Helper: Log file discovery ───────────────────────────────────

struct LogFile {
    name: String,
    path: PathBuf,
    size: u64,
}

fn scan_log_files(bin_dir: &PathBuf) -> Vec<LogFile> {
    let mut logs = Vec::new();

    // Nginx logs
    let nginx_log_dir = bin_dir.join("nginx").join("logs");
    if nginx_log_dir.exists() {
        if let Ok(entries) = fs::read_dir(&nginx_log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    logs.push(LogFile {
                        name: format!("nginx/{}", fname),
                        path,
                        size,
                    });
                }
            }
        }
    }

    // PHP logs (per version)
    let php_root = bin_dir.join("php");
    if php_root.exists() {
        if let Ok(entries) = fs::read_dir(&php_root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let ver = entry.file_name().to_string_lossy().to_string();
                    let log_path = entry.path().join("logs").join("php_errors.log");
                    if log_path.exists() {
                        let size = fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0);
                        logs.push(LogFile {
                            name: format!("php-{}/php_errors.log", ver),
                            path: log_path,
                            size,
                        });
                    }
                }
            }
        }
    }

    // MariaDB error log
    let mariadb_err = bin_dir.join("data").join("mariadb").join("mysql.err");
    if mariadb_err.exists() {
        let size = fs::metadata(&mariadb_err).map(|m| m.len()).unwrap_or(0);
        logs.push(LogFile {
            name: "mariadb/mysql.err".to_string(),
            path: mariadb_err,
            size,
        });
    }

    // Redis log
    let redis_log = bin_dir.join("redis").join("redis.log");
    if redis_log.exists() {
        let size = fs::metadata(&redis_log).map(|m| m.len()).unwrap_or(0);
        logs.push(LogFile {
            name: "redis/redis.log".to_string(),
            path: redis_log,
            size,
        });
    }

    // Mailpit log
    let mailpit_log = bin_dir.join("mailpit").join("mailpit.log");
    if mailpit_log.exists() {
        let size = fs::metadata(&mailpit_log).map(|m| m.len()).unwrap_or(0);
        logs.push(LogFile {
            name: "mailpit/mailpit.log".to_string(),
            path: mailpit_log,
            size,
        });
    }

    // Apache logs
    let apache_log_dir = bin_dir.join("apache").join("logs");
    if apache_log_dir.exists() {
        if let Ok(entries) = fs::read_dir(&apache_log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "log").unwrap_or(false) {
                    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    logs.push(LogFile {
                        name: format!("apache/{}", fname),
                        path,
                        size,
                    });
                }
            }
        }
    }

    // PostgreSQL logs
    let pg_data = bin_dir.join("data").join("postgres");
    if pg_data.exists() {
        for log_subdir in &["pg_log", "log"] {
            let pg_log_dir = pg_data.join(log_subdir);
            if pg_log_dir.exists() {
                if let Ok(entries) = fs::read_dir(&pg_log_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                        if ext == "log" || ext == "csv" {
                            let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                            logs.push(LogFile {
                                name: format!("postgresql/{}", fname),
                                path,
                                size,
                            });
                        }
                    }
                }
            }
        }
    }

    // MongoDB log
    let mongodb_data = bin_dir.join("data").join("mongodb");
    if mongodb_data.exists() {
        for log_dir_path in [&mongodb_data, &bin_dir.join("mongodb")] {
            let mongo_log = log_dir_path.join("mongod.log");
            if mongo_log.exists() {
                let size = fs::metadata(&mongo_log).map(|m| m.len()).unwrap_or(0);
                logs.push(LogFile {
                    name: "mongodb/mongod.log".to_string(),
                    path: mongo_log,
                    size,
                });
                break;
            }
        }
    }

    logs
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ─── Helper: Open in browser ──────────────────────────────────────

fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    { let _ = Command::new("cmd").args(["/C", "start", url]).spawn(); }
    #[cfg(target_os = "macos")]
    { let _ = Command::new("open").arg(url).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = Command::new("xdg-open").arg(url).spawn(); }
}

// ─── CLI Definition ───────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "orbit",
    about = "Orbit — Modern Local Development Environment",
    version = "1.1.0",
    author = "Orbit Dev Team",
    long_about = "Manage Nginx, PHP, MariaDB, Redis and more from the command line.\nThe modern alternative to XAMPP and Laragon."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status of all installed services
    Status,

    /// Start a service (or all with --all)
    Start {
        /// Service name to start (e.g., nginx, php-8.4, mariadb)
        service: Option<String>,
        /// Start all installed services
        #[arg(long)]
        all: bool,
    },

    /// Stop a service (or all with --all)
    Stop {
        /// Service name to stop
        service: Option<String>,
        /// Stop all running services
        #[arg(long)]
        all: bool,
    },

    /// Restart a service (or all with --all)
    Restart {
        /// Service name to restart
        service: Option<String>,
        /// Restart all services
        #[arg(long)]
        all: bool,
    },

    /// List all installed services
    List,

    /// List configured sites
    Sites {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show environment info and paths
    Info,

    /// Manage log files
    #[command(subcommand)]
    Logs(LogsCommands),

    /// Manage MariaDB databases
    #[command(subcommand)]
    Db(DbCommands),

    /// Open a site or tool in the browser
    Open {
        /// Domain name, 'adminer', or 'mailpit'
        target: String,
        /// Use HTTPS
        #[arg(long)]
        https: bool,
    },

    /// Manage PHP versions and extensions
    #[command(subcommand)]
    Php(PhpCommands),

    /// Manage the system hosts file
    #[command(subcommand)]
    Hosts(HostsCommands),

    /// Run composer via Orbit's PHP
    Composer {
        /// Arguments to pass to composer
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Install a service (e.g., orbit install nginx)
    Install {
        /// Service to install (nginx, php, mariadb, redis, nodejs, etc.)
        service: String,
        /// Version to install (e.g., 8.4 for PHP, 1.28 for Nginx)
        #[arg(long)]
        version: Option<String>,
    },

    /// Uninstall a service
    Uninstall {
        /// Service to uninstall
        service: String,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum LogsCommands {
    /// List all log files with sizes
    List,
    /// Show contents of a log file
    Show {
        /// Log name (e.g., nginx/access.log)
        name: String,
        /// Number of lines to show (from end)
        #[arg(short = 'n', default_value = "50")]
        lines: usize,
        /// Follow/tail mode (poll for new lines)
        #[arg(short = 'f', long = "tail")]
        follow: bool,
    },
    /// Clear a log file
    Clear {
        /// Log name to clear
        name: String,
    },
}

#[derive(Subcommand)]
enum DbCommands {
    /// List all databases
    List,
    /// Create a new database
    Create {
        /// Database name
        name: String,
    },
    /// Drop a database
    Drop {
        /// Database name
        name: String,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Export a database to SQL file
    Export {
        /// Database name
        name: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import a SQL file into a database
    Import {
        /// Database name
        name: String,
        /// SQL file path
        file: String,
    },
}

#[derive(Subcommand)]
enum PhpCommands {
    /// List installed PHP versions and ports
    List,
    /// Manage extensions for a PHP version
    Ext {
        /// PHP version (e.g., 8.4)
        version: String,
        /// Enable an extension
        #[arg(long)]
        enable: Option<String>,
        /// Disable an extension
        #[arg(long)]
        disable: Option<String>,
    },
}

#[derive(Subcommand)]
enum HostsCommands {
    /// List entries in the hosts file
    List,
    /// Add a domain to the hosts file
    Add {
        /// Domain to add (resolves to 127.0.0.1)
        domain: String,
    },
    /// Remove a domain from the hosts file
    Remove {
        /// Domain to remove
        domain: String,
    },
}

// ─── Command Handlers ─────────────────────────────────────────────

fn print_header() {
    println!();
    println!("  {} {}", "●".bright_green(), "Orbit".bold().white());
    println!("  {}", "Modern Local Development Environment".dimmed());
    println!();
}

fn cmd_status(bin_dir: &PathBuf) {
    print_header();

    let services = scan_services(bin_dir);

    if services.is_empty() {
        println!("  {} No services installed yet.", "!".yellow());
        println!("  {} Use the Orbit GUI to install services.", "→".dimmed());
        println!();
        return;
    }

    let name_width = services.iter().map(|s| s.name.len()).max().unwrap_or(10).max(10);
    let ver_width = services.iter().map(|s| s.version.len()).max().unwrap_or(8).max(8);

    println!("  {}", "SERVICES".dimmed().bold());
    println!("  {}", "─".repeat(name_width + ver_width + 25).dimmed());

    for svc in &services {
        let running = is_service_running(&svc.name);
        let port = get_service_port(&svc.name);

        let dot = if running { "●".bright_green() } else { "○".dimmed() };
        let name_colored = if running { svc.name.white().bold() } else { svc.name.dimmed() };
        let version_colored = svc.version.dimmed();
        let port_str = match port {
            Some(p) => format!(":{}", p),
            None => "—".to_string(),
        };
        let port_colored = if running { port_str.cyan() } else { port_str.dimmed() };
        let status_str = if running { "running".green() } else { "stopped".dimmed() };

        println!(
            "  {}  {:<width_n$}  {:<width_v$}  {:<8}  {}",
            dot,
            name_colored,
            version_colored,
            port_colored,
            status_str,
            width_n = name_width,
            width_v = ver_width,
        );
    }

    let running_count = services.iter().filter(|s| is_service_running(&s.name)).count();
    let total = services.len();

    println!("  {}", "─".repeat(name_width + ver_width + 25).dimmed());
    println!(
        "  {} {} / {} services running",
        "→".dimmed(),
        running_count.to_string().bright_green().bold(),
        total.to_string().white()
    );
    println!();
}

fn cmd_start(bin_dir: &PathBuf, service_name: Option<String>, all: bool) {
    let services = scan_services(bin_dir);

    if services.is_empty() {
        println!("  {} No services installed.", "✗".red());
        return;
    }

    let targets: Vec<&ServiceInfo> = if all {
        services.iter().filter(|s| {
            matches!(s.service_type.as_str(), "nginx" | "php" | "mariadb" | "redis" | "apache" | "mailpit" | "postgresql" | "mongodb")
        }).collect()
    } else if let Some(ref name) = service_name {
        services.iter().filter(|s| {
            s.name == *name || s.service_type == *name || s.name.starts_with(name)
                || (*name == "pg" && s.service_type == "postgresql")
                || (*name == "postgres" && s.service_type == "postgresql")
                || (*name == "mongo" && s.service_type == "mongodb")
                || (*name == "maria" && s.service_type == "mariadb")
                || (*name == "node" && s.service_type == "nodejs")
        }).collect()
    } else {
        println!("  {} Specify a service name or use --all", "!".yellow());
        println!("  {} orbit start nginx", "→".dimmed());
        println!("  {} orbit start --all", "→".dimmed());
        return;
    };

    if targets.is_empty() {
        if let Some(name) = service_name {
            println!("  {} Service '{}' not found.", "✗".red(), name);
        }
        return;
    }

    println!();
    for svc in &targets {
        if is_service_running(&svc.name) {
            println!("  {} {} already running", "—".dimmed(), svc.name.white());
            continue;
        }

        match start_service_process(svc) {
            Ok(pid) => {
                println!(
                    "  {} {} started (PID {})",
                    "✓".bright_green(),
                    svc.name.white().bold(),
                    pid.to_string().dimmed()
                );
            }
            Err(e) => {
                println!("  {} {} — {}", "✗".red(), svc.name.white(), e.dimmed());
            }
        }
    }
    println!();
}

fn cmd_stop(bin_dir: &PathBuf, service_name: Option<String>, all: bool) {
    let services = scan_services(bin_dir);

    let targets: Vec<&ServiceInfo> = if all {
        services.iter().collect()
    } else if let Some(ref name) = service_name {
        services.iter().filter(|s| {
            s.name == *name || s.service_type == *name || s.name.starts_with(name)
        }).collect()
    } else {
        println!("  {} Specify a service name or use --all", "!".yellow());
        return;
    };

    if targets.is_empty() {
        if let Some(name) = service_name {
            println!("  {} Service '{}' not found.", "✗".red(), name);
        }
        return;
    }

    println!();
    for svc in &targets {
        if !is_service_running(&svc.name) {
            println!("  {} {} not running", "—".dimmed(), svc.name.dimmed());
            continue;
        }

        match stop_service_process(&svc.name) {
            Ok(()) => {
                println!("  {} {} stopped", "✓".bright_green(), svc.name.white().bold());
            }
            Err(e) => {
                println!("  {} {} — {}", "✗".red(), svc.name.white(), e.dimmed());
            }
        }
    }
    println!();
}

fn cmd_restart(bin_dir: &PathBuf, service_name: Option<String>, all: bool) {
    let services = scan_services(bin_dir);

    let targets: Vec<&ServiceInfo> = if all {
        services.iter().filter(|s| {
            matches!(s.service_type.as_str(), "nginx" | "php" | "mariadb" | "redis" | "apache" | "mailpit" | "postgresql" | "mongodb")
        }).collect()
    } else if let Some(ref name) = service_name {
        services.iter().filter(|s| {
            s.name == *name || s.service_type == *name || s.name.starts_with(name)
                || (*name == "pg" && s.service_type == "postgresql")
                || (*name == "postgres" && s.service_type == "postgresql")
                || (*name == "mongo" && s.service_type == "mongodb")
                || (*name == "maria" && s.service_type == "mariadb")
                || (*name == "node" && s.service_type == "nodejs")
        }).collect()
    } else {
        println!("  {} Specify a service name or use --all", "!".yellow());
        return;
    };

    if targets.is_empty() {
        if let Some(name) = service_name {
            println!("  {} Service '{}' not found.", "✗".red(), name);
        }
        return;
    }

    println!();
    for svc in &targets {
        // Stop if running
        if is_service_running(&svc.name) {
            match stop_service_process(&svc.name) {
                Ok(()) => {
                    println!("  {} {} stopped", "↻".yellow(), svc.name.white());
                }
                Err(e) => {
                    println!("  {} {} stop failed — {}", "✗".red(), svc.name.white(), e.dimmed());
                    continue;
                }
            }
            // Brief pause for port release
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // Start
        match start_service_process(svc) {
            Ok(pid) => {
                println!(
                    "  {} {} restarted (PID {})",
                    "✓".bright_green(),
                    svc.name.white().bold(),
                    pid.to_string().dimmed()
                );
            }
            Err(e) => {
                println!("  {} {} start failed — {}", "✗".red(), svc.name.white(), e.dimmed());
            }
        }
    }
    println!();
}

fn cmd_list(bin_dir: &PathBuf) {
    print_header();

    let services = scan_services(bin_dir);

    let known_services = vec![
        ("nginx", "Nginx", "High-performance web server"),
        ("apache", "Apache", "Classic HTTP server"),
        ("php", "PHP", "Server-side scripting language"),
        ("mariadb", "MariaDB", "MySQL-compatible database"),
        ("postgresql", "PostgreSQL", "Advanced relational database"),
        ("mongodb", "MongoDB", "NoSQL document database"),
        ("redis", "Redis", "In-memory data store"),
        ("nodejs", "Node.js", "JavaScript runtime"),
        ("python", "Python", "General-purpose language"),
        ("bun", "Bun", "Fast JavaScript runtime"),
        ("go", "Go", "Compiled programming language"),
        ("deno", "Deno", "Next-gen JavaScript runtime"),
        ("rust", "Rust", "Systems programming language"),
        ("mailpit", "Mailpit", "Email testing tool"),
        ("composer", "Composer", "PHP dependency manager"),
    ];

    println!("  {}", "AVAILABLE SERVICES".dimmed().bold());
    println!("  {}", "─".repeat(55).dimmed());

    for (stype, label, description) in &known_services {
        let installed = services.iter().find(|s| s.service_type == *stype);
        let status = if let Some(svc) = installed {
            format!("{} {}", "✓".bright_green(), svc.version.dimmed())
        } else {
            format!("{}", "not installed".dimmed())
        };

        println!(
            "  {:<12} {:<30} {}",
            label.white().bold(),
            description.dimmed(),
            status
        );
    }

    println!("  {}", "─".repeat(55).dimmed());
    println!();
}

fn cmd_sites(json: bool) {
    let store = match read_sites_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  {} {}", "✗".red(), e);
            return;
        }
    };

    if json {
        // Machine-readable JSON output
        let sites_json: Vec<serde_json::Value> = store.sites.iter().map(|s| {
            serde_json::json!({
                "domain": s.domain,
                "path": s.path,
                "port": s.port,
                "php_version": s.php_version,
                "ssl_enabled": s.ssl_enabled,
                "web_server": s.web_server,
                "created_at": s.created_at,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&sites_json).unwrap_or_default());
        return;
    }

    print_header();

    if store.sites.is_empty() {
        println!("  {} No sites configured yet.", "!".yellow());
        println!("  {} Use the Orbit GUI to create sites.", "→".dimmed());
        println!();
        return;
    }

    let domain_width = store.sites.iter().map(|s| s.domain.len()).max().unwrap_or(15).max(15);

    println!("  {}", "SITES".dimmed().bold());
    println!("  {}", "─".repeat(domain_width + 50).dimmed());

    for site in &store.sites {
        let ssl_badge = if site.ssl_enabled { "🔒".to_string() } else { "  ".to_string() };
        let php_str = site.php_version.as_deref().unwrap_or("—");
        let ws_badge = match site.web_server.as_str() {
            "apache" => "Apache".yellow(),
            _ => "Nginx".green(),
        };

        println!(
            "  {} {:<width$}  :{:<5}  {:<8}  {:<8}  {}",
            ssl_badge,
            site.domain.white().bold(),
            site.port,
            php_str.dimmed(),
            ws_badge,
            site.path.dimmed(),
            width = domain_width,
        );
    }

    println!("  {}", "─".repeat(domain_width + 50).dimmed());
    println!(
        "  {} {} sites configured",
        "→".dimmed(),
        store.sites.len().to_string().bright_green().bold()
    );
    println!();
}

fn cmd_info(bin_dir: &PathBuf) {
    print_header();

    let data_dir = get_orbit_data_dir();
    let config_dir = get_config_dir();

    println!("  {}", "PATHS".dimmed().bold());
    println!("  {}", "─".repeat(50).dimmed());
    println!("  {:<16} {}", "Data Dir:".white().bold(), data_dir.display().to_string().cyan());
    println!("  {:<16} {}", "Bin Dir:".white().bold(), bin_dir.display().to_string().cyan());
    println!("  {:<16} {}", "Config Dir:".white().bold(), config_dir.display().to_string().cyan());
    println!();

    let services = scan_services(bin_dir);

    if !services.is_empty() {
        println!("  {}", "INSTALLED".dimmed().bold());
        println!("  {}", "─".repeat(50).dimmed());
        for svc in &services {
            let running = is_service_running(&svc.name);
            let status = if running { "●".bright_green() } else { "○".dimmed() };
            println!(
                "  {}  {:<16} {}",
                status,
                svc.name.white().bold(),
                svc.version.dimmed()
            );
        }
        println!();
    }

    // Sites count
    if let Ok(store) = read_sites_store() {
        if !store.sites.is_empty() {
            println!("  {} {} sites configured", "→".dimmed(), store.sites.len().to_string().white());
            println!();
        }
    }
}

// ─── Log Commands ─────────────────────────────────────────────────

fn cmd_logs_list(bin_dir: &PathBuf) {
    print_header();

    let logs = scan_log_files(bin_dir);

    if logs.is_empty() {
        println!("  {} No log files found.", "!".yellow());
        println!();
        return;
    }

    let name_width = logs.iter().map(|l| l.name.len()).max().unwrap_or(20).max(20);

    println!("  {}", "LOG FILES".dimmed().bold());
    println!("  {}", "─".repeat(name_width + 20).dimmed());

    for log in &logs {
        let size_str = format_size(log.size);
        let name_colored = if log.size > 0 { log.name.white().bold() } else { log.name.dimmed() };
        println!(
            "  {:<width$}  {}",
            name_colored,
            size_str.dimmed(),
            width = name_width,
        );
    }

    println!("  {}", "─".repeat(name_width + 20).dimmed());
    println!();
}

fn cmd_logs_show(bin_dir: &PathBuf, name: &str, lines: usize, follow: bool) {
    let logs = scan_log_files(bin_dir);

    let log = logs.iter().find(|l| l.name == name || l.name.ends_with(name));
    let log = match log {
        Some(l) => l,
        None => {
            eprintln!("  {} Log '{}' not found. Use 'orbit logs list' to see available logs.", "✗".red(), name);
            return;
        }
    };

    if !log.path.exists() {
        eprintln!("  {} Log file does not exist: {}", "✗".red(), log.path.display());
        return;
    }

    // Read last N lines
    let content = match fs::read_to_string(&log.path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  {} Failed to read log: {}", "✗".red(), e);
            return;
        }
    };

    let all_lines: Vec<&str> = content.lines().collect();
    let start = if all_lines.len() > lines { all_lines.len() - lines } else { 0 };

    println!("{} {} (last {} lines)", "─".dimmed(), log.name.white().bold(), lines);
    for line in &all_lines[start..] {
        println!("{}", line);
    }

    if follow {
        println!("{}", "─ Following (Ctrl+C to stop) ─".dimmed());
        // Poll-based tail
        let mut file = match std::fs::File::open(&log.path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("  {} Failed to open log for tailing: {}", "✗".red(), e);
                return;
            }
        };
        let _ = file.seek(SeekFrom::End(0));
        let mut reader = BufReader::new(file);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // No new data, sleep
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
                Ok(_) => {
                    print!("{}", line);
                }
                Err(_) => break,
            }
        }
    }
}

fn cmd_logs_clear(bin_dir: &PathBuf, name: &str) {
    let logs = scan_log_files(bin_dir);

    let log = logs.iter().find(|l| l.name == name || l.name.ends_with(name));
    let log = match log {
        Some(l) => l,
        None => {
            eprintln!("  {} Log '{}' not found.", "✗".red(), name);
            return;
        }
    };

    match fs::write(&log.path, "") {
        Ok(_) => println!("  {} {} cleared", "✓".bright_green(), log.name.white().bold()),
        Err(e) => eprintln!("  {} Failed to clear {}: {}", "✗".red(), log.name, e),
    }
}

// ─── Database Commands ────────────────────────────────────────────

fn cmd_db_list(bin_dir: &PathBuf) {
    let client = match find_mariadb_client(bin_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  {} {}", "✗".red(), e);
            return;
        }
    };

    let output = hidden_command(&client)
        .arg("--host=127.0.0.1")
        .arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg("-e").arg("SHOW DATABASES")
        .arg("--batch").arg("--skip-column-names")
        .output();

    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!("  {} {}", "✗".red(), stderr.trim());
                return;
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            let system_dbs = ["information_schema", "performance_schema", "mysql", "sys"];

            print_header();
            println!("  {}", "DATABASES".dimmed().bold());
            println!("  {}", "─".repeat(40).dimmed());

            let mut count = 0;
            for db in stdout.lines() {
                let db = db.trim();
                if db.is_empty() { continue; }
                let is_system = system_dbs.contains(&db);
                if is_system {
                    println!("  {}  {} {}", "○".dimmed(), db.dimmed(), "(system)".dimmed());
                } else {
                    println!("  {}  {}", "●".bright_green(), db.white().bold());
                    count += 1;
                }
            }

            println!("  {}", "─".repeat(40).dimmed());
            println!("  {} {} user databases", "→".dimmed(), count.to_string().bright_green().bold());
            println!();
        }
        Err(e) => {
            eprintln!("  {} Failed to connect to MariaDB: {}", "✗".red(), e);
            eprintln!("  {} Is MariaDB running?", "→".dimmed());
        }
    }
}

fn cmd_db_create(bin_dir: &PathBuf, name: &str) {
    let client = match find_mariadb_client(bin_dir) {
        Ok(c) => c,
        Err(e) => { eprintln!("  {} {}", "✗".red(), e); return; }
    };

    let sql = format!("CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci", name);
    let output = hidden_command(&client)
        .arg("--host=127.0.0.1").arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg("-e").arg(&sql)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            println!("  {} Database '{}' created", "✓".bright_green(), name.white().bold());
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!("  {} {}", "✗".red(), stderr.trim());
        }
        Err(e) => eprintln!("  {} {}", "✗".red(), e),
    }
}

fn cmd_db_drop(bin_dir: &PathBuf, name: &str, yes: bool) {
    let system_dbs = ["information_schema", "performance_schema", "mysql", "sys"];
    if system_dbs.contains(&name) {
        eprintln!("  {} Cannot drop system database '{}'", "✗".red(), name);
        return;
    }

    if !yes {
        eprint!("  {} Drop database '{}'? This cannot be undone. [y/N] ", "!".yellow(), name);
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("  {} Cancelled", "—".dimmed());
            return;
        }
    }

    let client = match find_mariadb_client(bin_dir) {
        Ok(c) => c,
        Err(e) => { eprintln!("  {} {}", "✗".red(), e); return; }
    };

    let sql = format!("DROP DATABASE `{}`", name);
    let output = hidden_command(&client)
        .arg("--host=127.0.0.1").arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg("-e").arg(&sql)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            println!("  {} Database '{}' dropped", "✓".bright_green(), name.white().bold());
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!("  {} {}", "✗".red(), stderr.trim());
        }
        Err(e) => eprintln!("  {} {}", "✗".red(), e),
    }
}

fn cmd_db_export(bin_dir: &PathBuf, name: &str, output_path: Option<String>) {
    let dump_exe = match find_mariadb_dump(bin_dir) {
        Ok(d) => d,
        Err(e) => { eprintln!("  {} {}", "✗".red(), e); return; }
    };

    let out_file = output_path.unwrap_or_else(|| format!("{}.sql", name));

    let output = hidden_command(&dump_exe)
        .arg("--host=127.0.0.1").arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg("--routines").arg("--triggers").arg("--single-transaction")
        .arg(name)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            match fs::write(&out_file, &out.stdout) {
                Ok(_) => {
                    println!(
                        "  {} Database '{}' exported to {} ({})",
                        "✓".bright_green(),
                        name.white().bold(),
                        out_file.cyan(),
                        format_size(out.stdout.len() as u64).dimmed()
                    );
                }
                Err(e) => eprintln!("  {} Failed to write file: {}", "✗".red(), e),
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!("  {} Export failed: {}", "✗".red(), stderr.trim());
        }
        Err(e) => eprintln!("  {} {}", "✗".red(), e),
    }
}

fn cmd_db_import(bin_dir: &PathBuf, name: &str, file: &str) {
    if !std::path::Path::new(file).exists() {
        eprintln!("  {} SQL file not found: {}", "✗".red(), file);
        return;
    }

    let client = match find_mariadb_client(bin_dir) {
        Ok(c) => c,
        Err(e) => { eprintln!("  {} {}", "✗".red(), e); return; }
    };

    let sql_content = match fs::read(file) {
        Ok(c) => c,
        Err(e) => { eprintln!("  {} Failed to read SQL file: {}", "✗".red(), e); return; }
    };

    let file_size = sql_content.len();

    let mut child = match hidden_command(&client)
        .arg("--host=127.0.0.1").arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg(name)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => { eprintln!("  {} Failed to start mysql client: {}", "✗".red(), e); return; }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&sql_content);
    }

    match child.wait_with_output() {
        Ok(out) if out.status.success() => {
            println!(
                "  {} Imported {} into '{}'",
                "✓".bright_green(),
                format_size(file_size as u64).dimmed(),
                name.white().bold()
            );
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!("  {} Import failed: {}", "✗".red(), stderr.trim());
        }
        Err(e) => eprintln!("  {} {}", "✗".red(), e),
    }
}

// ─── Open Command ─────────────────────────────────────────────────

fn cmd_open(target: &str, https: bool) {
    match target.to_lowercase().as_str() {
        "adminer" => {
            open_in_browser("http://localhost:8080");
            println!("  {} Opening Adminer...", "✓".bright_green());
        }
        "mailpit" => {
            open_in_browser("http://localhost:8025");
            println!("  {} Opening Mailpit...", "✓".bright_green());
        }
        _ => {
            // Try to find in sites
            if let Ok(store) = read_sites_store() {
                if let Some(site) = store.sites.iter().find(|s| s.domain == target) {
                    let proto = if https || site.ssl_enabled { "https" } else { "http" };
                    let port_suffix = if (site.port == 80 && !site.ssl_enabled) || (site.port == 443 && site.ssl_enabled) {
                        String::new()
                    } else {
                        format!(":{}", site.port)
                    };
                    let url = format!("{}://{}{}", proto, site.domain, port_suffix);
                    open_in_browser(&url);
                    println!("  {} Opening {}...", "✓".bright_green(), url.cyan());
                    return;
                }
            }
            // Fallback: treat as domain directly
            let proto = if https { "https" } else { "http" };
            let url = format!("{}://{}", proto, target);
            open_in_browser(&url);
            println!("  {} Opening {}...", "✓".bright_green(), url.cyan());
        }
    }
}

// ─── PHP Commands ─────────────────────────────────────────────────

fn cmd_php_list(bin_dir: &PathBuf) {
    print_header();

    let php_root = bin_dir.join("php");
    if !php_root.exists() {
        println!("  {} No PHP versions installed.", "!".yellow());
        println!();
        return;
    }

    println!("  {}", "PHP VERSIONS".dimmed().bold());
    println!("  {}", "─".repeat(50).dimmed());

    if let Ok(entries) = fs::read_dir(&php_root) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let ver_dir = entry.file_name().to_string_lossy().to_string();
            let exe_path = entry.path().join("php-cgi.exe");
            if !exe_path.exists() { continue; }

            let version = parse_version_output(&exe_path, &["-v"], "PHP ", 4);
            let name = format!("php-{}", ver_dir);
            let port = get_service_port(&name);
            let running = is_service_running(&name);

            let dot = if running { "●".bright_green() } else { "○".dimmed() };
            let port_str = port.map(|p| format!(":{}", p)).unwrap_or_else(|| "—".to_string());

            // Count extensions
            let ini_path = entry.path().join("php.ini");
            let ext_count = if ini_path.exists() {
                fs::read_to_string(&ini_path)
                    .map(|c| c.lines().filter(|l| l.starts_with("extension=")).count())
                    .unwrap_or(0)
            } else {
                0
            };

            println!(
                "  {}  {:<12}  {:<10}  {:<8}  {} extensions",
                dot,
                ver_dir.white().bold(),
                version.dimmed(),
                if running { port_str.cyan() } else { port_str.dimmed() },
                ext_count.to_string().dimmed()
            );
        }
    }

    println!("  {}", "─".repeat(50).dimmed());
    println!();
}

fn cmd_php_ext(bin_dir: &PathBuf, version: &str, enable: Option<String>, disable: Option<String>) {
    let php_dir = bin_dir.join("php").join(version);
    let ini_path = php_dir.join("php.ini");

    if !ini_path.exists() {
        eprintln!("  {} PHP {} not found or php.ini missing", "✗".red(), version);
        return;
    }

    let content = match fs::read_to_string(&ini_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("  {} Failed to read php.ini: {}", "✗".red(), e); return; }
    };

    if let Some(ref ext_name) = enable {
        // Enable: uncomment or add extension line
        let pattern = format!(";extension={}", ext_name);
        let replacement = format!("extension={}", ext_name);

        let new_content = if content.contains(&pattern) {
            content.replace(&pattern, &replacement)
        } else if content.contains(&replacement) {
            println!("  {} Extension '{}' already enabled", "—".dimmed(), ext_name);
            return;
        } else {
            format!("{}\nextension={}\n", content.trim_end(), ext_name)
        };

        match fs::write(&ini_path, new_content) {
            Ok(_) => println!("  {} Extension '{}' enabled. Restart PHP to apply.", "✓".bright_green(), ext_name.white().bold()),
            Err(e) => eprintln!("  {} Failed to write php.ini: {}", "✗".red(), e),
        }
        return;
    }

    if let Some(ref ext_name) = disable {
        // Disable: comment out extension line
        let pattern = format!("extension={}", ext_name);
        let replacement = format!(";extension={}", ext_name);

        if !content.contains(&pattern) {
            println!("  {} Extension '{}' not found or already disabled", "—".dimmed(), ext_name);
            return;
        }

        let new_content = content.replace(&pattern, &replacement);
        match fs::write(&ini_path, new_content) {
            Ok(_) => println!("  {} Extension '{}' disabled. Restart PHP to apply.", "✓".bright_green(), ext_name.white().bold()),
            Err(e) => eprintln!("  {} Failed to write php.ini: {}", "✗".red(), e),
        }
        return;
    }

    // No --enable or --disable: list extensions
    println!();
    println!("  {} PHP {} Extensions", "●".bright_green(), version.white().bold());
    println!("  {}", "─".repeat(40).dimmed());

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("extension=") {
            let ext = trimmed.strip_prefix("extension=").unwrap_or("");
            println!("  {} {}", "✓".bright_green(), ext.white());
        } else if trimmed.starts_with(";extension=") {
            let ext = trimmed.strip_prefix(";extension=").unwrap_or("");
            println!("  {} {}", "○".dimmed(), ext.dimmed());
        }
    }
    println!();
}

// ─── Hosts Commands ───────────────────────────────────────────────

fn get_hosts_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts") }
    #[cfg(not(target_os = "windows"))]
    { PathBuf::from("/etc/hosts") }
}

fn cmd_hosts_list() {
    let hosts_path = get_hosts_path();
    let content = match fs::read_to_string(&hosts_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  {} Failed to read hosts file: {}", "✗".red(), e);
            return;
        }
    };

    print_header();
    println!("  {}", "HOSTS FILE".dimmed().bold());
    println!("  {}", "─".repeat(50).dimmed());

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            let ip = parts[0];
            let domain = parts[1];
            let is_local = ip == "127.0.0.1" || ip == "::1";
            if is_local {
                println!("  {}  {:<16} {}", "●".bright_green(), ip.dimmed(), domain.white().bold());
            } else {
                println!("  {}  {:<16} {}", "○".dimmed(), ip.dimmed(), domain.dimmed());
            }
        }
    }
    println!("  {}", "─".repeat(50).dimmed());
    println!();
}

fn cmd_hosts_add(domain: &str) {
    let hosts_path = get_hosts_path();
    let content = match fs::read_to_string(&hosts_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  {} Failed to read hosts file: {}", "✗".red(), e);
            return;
        }
    };

    // Check if already exists
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') { continue; }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == domain {
            println!("  {} Domain '{}' already in hosts file", "—".dimmed(), domain);
            return;
        }
    }

    let entry = format!("\n127.0.0.1  {}\n", domain);
    let mut new_content = content;
    new_content.push_str(&entry);

    match fs::write(&hosts_path, new_content) {
        Ok(_) => println!("  {} Added '{}' to hosts file", "✓".bright_green(), domain.white().bold()),
        Err(_) => {
            eprintln!("  {} Failed to write hosts file. Run as Administrator.", "✗".red());
            eprintln!("  {} Try: orbit hosts add {} (in elevated terminal)", "→".dimmed(), domain);
        }
    }
}

fn cmd_hosts_remove(domain: &str) {
    let hosts_path = get_hosts_path();
    let content = match fs::read_to_string(&hosts_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  {} Failed to read hosts file: {}", "✗".red(), e);
            return;
        }
    };

    let new_lines: Vec<&str> = content.lines().filter(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() { return true; }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        !(parts.len() >= 2 && parts[1] == domain)
    }).collect();

    let new_content = new_lines.join("\n");

    match fs::write(&hosts_path, new_content) {
        Ok(_) => println!("  {} Removed '{}' from hosts file", "✓".bright_green(), domain.white().bold()),
        Err(_) => {
            eprintln!("  {} Failed to write hosts file. Run as Administrator.", "✗".red());
        }
    }
}

// ─── Composer Command ─────────────────────────────────────────────

fn cmd_composer(bin_dir: &PathBuf, args: Vec<String>) {
    let composer_phar = bin_dir.join("composer").join("composer.phar");
    if !composer_phar.exists() {
        eprintln!("  {} Composer not installed. Install it from the Orbit GUI.", "✗".red());
        return;
    }

    // Find the first available PHP
    let php_root = bin_dir.join("php");
    let mut php_exe: Option<PathBuf> = None;
    if php_root.exists() {
        if let Ok(entries) = fs::read_dir(&php_root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let exe = entry.path().join("php.exe");
                    if exe.exists() {
                        php_exe = Some(exe);
                        break;
                    }
                }
            }
        }
    }

    let php = match php_exe {
        Some(p) => p,
        None => {
            eprintln!("  {} No PHP version installed. Install PHP from the Orbit GUI.", "✗".red());
            return;
        }
    };

    let status = Command::new(&php)
        .arg(composer_phar.to_string_lossy().to_string())
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    match status {
        Ok(s) => {
            if !s.success() {
                std::process::exit(s.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("  {} Failed to run composer: {}", "✗".red(), e);
            std::process::exit(1);
        }
    }
}

// ─── Registry Types ───────────────────────────────────────────────

/// Embedded fallback registry
const CLI_FALLBACK_REGISTRY: &str = include_str!("../dist/libraries.json");

#[derive(Deserialize)]
struct RegistryPlatformDownload {
    url: String,
    filename: String,
}

#[derive(Deserialize)]
struct RegistryVersionInfo {
    #[allow(dead_code)]
    latest: String,
    #[serde(default)]
    windows: Option<RegistryPlatformDownload>,
    #[serde(default)]
    all_platforms: Option<RegistryPlatformDownload>,
}

#[derive(Deserialize)]
struct RegistryServiceInfo {
    name: String,
    #[serde(default, rename = "availableVersions")]
    available_versions: Option<Vec<String>>,
    #[serde(default)]
    versions: Option<std::collections::HashMap<String, RegistryVersionInfo>>,
    // Single-version services
    #[serde(default)]
    windows: Option<RegistryPlatformDownload>,
    #[serde(default)]
    all_platforms: Option<RegistryPlatformDownload>,
}

#[derive(Deserialize)]
struct CliRegistry {
    services: std::collections::HashMap<String, RegistryServiceInfo>,
}

fn load_registry() -> Result<CliRegistry, String> {
    // Try remote first, fall back to embedded
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok();

    if let Some(client) = client {
        if let Ok(resp) = client.get("https://raw.githubusercontent.com/alinsgit/orbit-libraries/main/dist/libraries.json").send() {
            if resp.status().is_success() {
                if let Ok(text) = resp.text() {
                    if let Ok(registry) = serde_json::from_str::<CliRegistry>(&text) {
                        return Ok(registry);
                    }
                }
            }
        }
    }

    // Fallback to embedded
    serde_json::from_str(CLI_FALLBACK_REGISTRY)
        .map_err(|e| format!("Failed to parse registry: {}", e))
}

fn get_download_info(service_info: &RegistryServiceInfo, version: Option<&str>) -> Option<(String, String)> {
    // Multi-version service
    if let Some(versions) = &service_info.versions {
        let ver = version.or_else(|| {
            service_info.available_versions.as_ref()
                .and_then(|v| v.first())
                .map(|s| s.as_str())
        })?;

        let version_info = versions.get(ver)?;

        #[cfg(target_os = "windows")]
        if let Some(dl) = &version_info.windows {
            return Some((dl.url.clone(), dl.filename.clone()));
        }
        if let Some(dl) = &version_info.all_platforms {
            return Some((dl.url.clone(), dl.filename.clone()));
        }
        return None;
    }

    // Single-version service
    #[cfg(target_os = "windows")]
    if let Some(dl) = &service_info.windows {
        return Some((dl.url.clone(), dl.filename.clone()));
    }
    if let Some(dl) = &service_info.all_platforms {
        return Some((dl.url.clone(), dl.filename.clone()));
    }
    None
}

// ─── Install/Uninstall Commands ──────────────────────────────────

fn cmd_install(bin_dir: &PathBuf, service: &str, version: Option<String>) {
    println!();

    // Normalize service name
    let service_key = match service {
        "node" => "nodejs",
        "pg" | "postgres" => "postgresql",
        "mongo" => "mongodb",
        "maria" => "mariadb",
        s => s,
    };

    // Load registry
    print!("  {} Fetching registry...", "⟳".dimmed());
    std::io::stdout().flush().unwrap_or(());
    let registry = match load_registry() {
        Ok(r) => {
            print!("\r");
            std::io::stdout().flush().unwrap_or(());
            r
        }
        Err(e) => {
            println!("\r  {} Failed to load registry: {}", "✗".red(), e);
            return;
        }
    };

    let service_info = match registry.services.get(service_key) {
        Some(info) => info,
        None => {
            println!("  {} Service '{}' not found in registry.", "✗".red(), service);
            println!("  {} Available services:", "→".dimmed());
            let mut names: Vec<&String> = registry.services.keys().collect();
            names.sort();
            for name in names {
                println!("      {}", name.white());
            }
            println!();
            return;
        }
    };

    // For PHP, build the service_type with version prefix
    let install_type = if service_key == "php" {
        let ver = version.as_deref().unwrap_or_else(|| {
            service_info.available_versions.as_ref()
                .and_then(|v| v.first())
                .map(|s| s.as_str())
                .unwrap_or("8.4")
        });
        format!("php-{}", ver)
    } else {
        service_key.to_string()
    };

    // Get download URL
    let (url, filename) = match get_download_info(service_info, version.as_deref()) {
        Some(info) => info,
        None => {
            println!("  {} No download available for '{}' on this platform.", "✗".red(), service);
            if let Some(versions) = &service_info.available_versions {
                println!("  {} Available versions: {}", "→".dimmed(), versions.join(", "));
            }
            println!();
            return;
        }
    };

    let ver_display = version.as_deref().unwrap_or("latest");
    println!("  {} Installing {} {}...", "→".dimmed(), service_info.name.white().bold(), ver_display.dimmed());
    println!("  {} {}", "↓".dimmed(), url.dimmed());

    // Create downloads directory
    let downloads_dir = bin_dir.join("downloads");
    fs::create_dir_all(&downloads_dir).ok();

    let dest_path = downloads_dir.join(&filename);

    // Download
    print!("  {} Downloading...", "⟳".dimmed());
    std::io::stdout().flush().unwrap_or(());

    match cli_download_file(&url, &dest_path) {
        Ok(size) => {
            println!("\r  {} Downloaded ({})", "✓".bright_green(), format_size(size));
        }
        Err(e) => {
            println!("\r  {} Download failed: {}", "✗".red(), e);
            return;
        }
    }

    // Determine extraction target and strip_root
    let (extract_target, strip_root) = match install_type.as_str() {
        "nginx" => (bin_dir.join("nginx"), true),
        "mariadb" => (bin_dir.join("mariadb"), true),
        "postgresql" => (bin_dir.join("postgresql"), true),
        "mongodb" => (bin_dir.join("mongodb"), true),
        "nodejs" => (bin_dir.join("nodejs"), true),
        "bun" => (bin_dir.join("bun"), true),
        "apache" => (bin_dir.join("apache"), true),
        "go" => (bin_dir.join("go"), true),
        "redis" => (bin_dir.join("redis"), true),
        "deno" => (bin_dir.join("deno"), false),
        "python" => (bin_dir.join("python"), false),
        "mailpit" => (bin_dir.join("mailpit"), true),
        s if s.starts_with("php-") => {
            let ver = s.strip_prefix("php-").unwrap_or("latest");
            (bin_dir.join("php").join(ver), false)
        }
        "composer" => (bin_dir.join("composer"), false),
        "rust" => (bin_dir.join("rust"), false),
        _ => (bin_dir.join("misc").join(service_key), false),
    };

    // Handle raw executables (rust, composer)
    if service_key == "rust" {
        fs::create_dir_all(&extract_target).ok();
        let target_exe = extract_target.join(&filename);
        match fs::copy(&dest_path, &target_exe) {
            Ok(_) => {
                let _ = fs::remove_file(&dest_path);
                println!("  {} Installed to {}", "✓".bright_green(), extract_target.display().to_string().cyan());
                println!();
                return;
            }
            Err(e) => {
                println!("  {} Failed to copy executable: {}", "✗".red(), e);
                return;
            }
        }
    }

    if service_key == "composer" {
        fs::create_dir_all(&extract_target).ok();
        let target_phar = extract_target.join("composer.phar");
        match fs::copy(&dest_path, &target_phar) {
            Ok(_) => {
                let _ = fs::remove_file(&dest_path);
                println!("  {} Installed to {}", "✓".bright_green(), extract_target.display().to_string().cyan());
                println!();
                return;
            }
            Err(e) => {
                println!("  {} Failed to copy composer.phar: {}", "✗".red(), e);
                return;
            }
        }
    }

    // Clean and create extraction target
    if extract_target.exists() {
        let _ = fs::remove_dir_all(&extract_target);
    }
    fs::create_dir_all(&extract_target).ok();

    // Extract
    print!("  {} Extracting...", "⟳".dimmed());
    std::io::stdout().flush().unwrap_or(());

    match cli_extract_zip(&dest_path, &extract_target, strip_root) {
        Ok(_) => {
            let _ = fs::remove_file(&dest_path);
            println!("\r  {} Extracted successfully", "✓".bright_green());
        }
        Err(e) => {
            println!("\r  {} Extraction failed: {}", "✗".red(), e);
            return;
        }
    }

    // Post-install configuration
    if install_type.starts_with("php-") {
        match cli_configure_php(&extract_target) {
            Ok(_) => println!("  {} PHP configured (php.ini created)", "✓".bright_green()),
            Err(e) => println!("  {} PHP config warning: {}", "!".yellow(), e),
        }
    } else if service_key == "apache" {
        // Apache24 subfolder handling
        let apache24 = extract_target.join("Apache24");
        if apache24.exists() {
            cli_move_subfolder_up(&apache24, &extract_target).ok();
        }
        match cli_configure_apache(&extract_target) {
            Ok(_) => println!("  {} Apache configured (httpd.conf updated)", "✓".bright_green()),
            Err(e) => println!("  {} Apache config warning: {}", "!".yellow(), e),
        }
    }
    // Note: PostgreSQL ZIP extracts to postgresql/pgsql/bin/ (nested).
    // Scanner handles both flattened and nested structures.

    // Verify installation
    let services = scan_services(bin_dir);
    let found = services.iter().find(|s| {
        s.service_type == service_key || s.name == install_type
    });

    if let Some(svc) = found {
        println!("  {} {} {} installed successfully",
            "✓".bright_green(),
            svc.name.white().bold(),
            svc.version.dimmed()
        );
    } else {
        println!("  {} Installed to {}", "✓".bright_green(), extract_target.display().to_string().cyan());
    }
    println!();
}

fn cmd_uninstall(bin_dir: &PathBuf, service: &str, skip_confirm: bool) {
    println!();

    let service_key = match service {
        "node" => "nodejs",
        "pg" | "postgres" => "postgresql",
        "mongo" => "mongodb",
        "maria" => "mariadb",
        s => s,
    };

    // Find the service directory
    let service_dir = if service_key.starts_with("php-") {
        let ver = service_key.strip_prefix("php-").unwrap_or("8.4");
        bin_dir.join("php").join(ver)
    } else {
        bin_dir.join(service_key)
    };

    if !service_dir.exists() {
        println!("  {} Service '{}' is not installed.", "✗".red(), service);
        println!();
        return;
    }

    if !skip_confirm {
        print!("  {} Remove {} at {}? [y/N] ",
            "?".yellow(),
            service.white().bold(),
            service_dir.display().to_string().dimmed()
        );
        std::io::stdout().flush().unwrap_or(());

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            println!("  {} Cancelled.", "—".dimmed());
            return;
        }
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("  {} Cancelled.", "—".dimmed());
            println!();
            return;
        }
    }

    // Stop service if running
    if is_service_running(service_key) {
        match stop_service_process(service_key) {
            Ok(()) => println!("  {} Stopped {}", "✓".bright_green(), service_key.white()),
            Err(_) => println!("  {} Could not stop {} (may need manual cleanup)", "!".yellow(), service_key),
        }
    }

    // Remove directory
    match fs::remove_dir_all(&service_dir) {
        Ok(_) => {
            println!("  {} {} uninstalled successfully", "✓".bright_green(), service.white().bold());
        }
        Err(e) => {
            println!("  {} Failed to remove {}: {}", "✗".red(), service_dir.display(), e);
        }
    }
    println!();
}

// ─── CLI Download & Extract ──────────────────────────────────────

fn cli_download_file(url: &str, dest_path: &PathBuf) -> Result<u64, String> {
    // Try mirror URL for MariaDB
    let urls_to_try: Vec<String> = {
        let mut urls = vec![url.to_string()];
        if url.contains("downloads.mariadb.org/f/") {
            if let Some(after_f) = url.split("downloads.mariadb.org/f/").nth(1) {
                urls.push(format!("https://mirror.kumi.systems/mariadb/{}", after_f));
            }
        }
        urls
    };

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut last_error = String::new();

    for attempt_url in &urls_to_try {
        let resp = match client.get(attempt_url)
            .header("Accept", "application/octet-stream, application/zip, */*;q=0.1")
            .header("Accept-Encoding", "identity")
            .send()
        {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("Request failed: {}", e);
                continue;
            }
        };

        if !resp.status().is_success() {
            last_error = format!("HTTP {}", resp.status());
            continue;
        }

        // Reject HTML responses
        if let Some(ct) = resp.headers().get("content-type") {
            if ct.to_str().unwrap_or("").contains("text/html") {
                last_error = "Received HTML instead of file".to_string();
                continue;
            }
        }

        let total_size = resp.content_length().unwrap_or(0);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let bytes = resp.bytes()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        fs::write(dest_path, &bytes)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        let written = if total_size > 0 { total_size } else { bytes.len() as u64 };
        return Ok(written);
    }

    Err(last_error)
}

fn cli_extract_zip(zip_path: &PathBuf, extract_path: &PathBuf, strip_root: bool) -> Result<(), String> {
    use zip::ZipArchive;

    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip: {}", e))?;

    // Detect root folder
    let root_folder = if strip_root && archive.len() > 0 {
        if let Ok(first) = archive.by_index(0) {
            let name = first.name().to_string();
            if name.ends_with('/') {
                Some(name)
            } else if let Some(idx) = name.find('/') {
                Some(format!("{}/", &name[..idx]))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;

        let file_path = match entry.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => continue,
        };

        let relative_path = if let Some(ref root) = root_folder {
            let path_str = file_path.to_string_lossy();
            if path_str.starts_with(root) {
                PathBuf::from(&path_str[root.len()..])
            } else {
                file_path
            }
        } else {
            file_path
        };

        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let outpath = extract_path.join(&relative_path);

        if entry.name().ends_with('/') {
            fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).ok();
                }
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)
                .map_err(|e| format!("Failed to read entry: {}", e))?;
            outfile.write_all(&buf)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }
    }

    Ok(())
}

fn cli_move_subfolder_up(source: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let entries = fs::read_dir(source)
        .map_err(|e| format!("Failed to read subfolder: {}", e))?;

    for entry in entries.flatten() {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dest.join(&file_name);

        if dest_path.exists() && dest_path != src_path {
            continue;
        }

        if src_path.is_dir() {
            cli_copy_dir_all(&src_path, &dest_path)?;
            let _ = fs::remove_dir_all(&src_path);
        } else {
            fs::rename(&src_path, &dest_path)
                .map_err(|e| format!("Failed to move {}: {}", file_name.to_string_lossy(), e))?;
        }
    }

    let _ = fs::remove_dir(source);
    Ok(())
}

fn cli_copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Failed to create dir: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            cli_copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {:?}: {}", src_path, e))?;
        }
    }
    Ok(())
}

fn cli_configure_php(php_path: &PathBuf) -> Result<(), String> {
    let ini_dev = php_path.join("php.ini-development");
    let ini_prod = php_path.join("php.ini-production");
    let ini_target = php_path.join("php.ini");

    if !ini_target.exists() {
        if ini_dev.exists() {
            fs::copy(&ini_dev, &ini_target).map_err(|e| format!("Failed to create php.ini: {}", e))?;
        } else if ini_prod.exists() {
            fs::copy(&ini_prod, &ini_target).map_err(|e| format!("Failed to create php.ini: {}", e))?;
        } else {
            return Err("No php.ini template found".to_string());
        }
    }

    let mut content = fs::read_to_string(&ini_target)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    // Set extension_dir
    let ext_dir = php_path.join("ext");
    let ext_dir_str = ext_dir.to_string_lossy().replace('\\', "/");

    if content.contains(";extension_dir = \"ext\"") {
        content = content.replace(
            ";extension_dir = \"ext\"",
            &format!("extension_dir = \"{}\"", ext_dir_str)
        );
    }

    // Enable common extensions
    for ext in ["curl", "fileinfo", "gd", "mbstring", "mysqli", "openssl", "pdo_mysql", "zip"] {
        let disabled = format!(";extension={}", ext);
        let enabled = format!("extension={}", ext);
        if content.contains(&disabled) {
            content = content.replace(&disabled, &enabled);
        }
    }

    fs::write(&ini_target, content).map_err(|e| format!("Failed to write php.ini: {}", e))?;
    Ok(())
}

fn cli_configure_apache(apache_path: &PathBuf) -> Result<(), String> {
    let conf_dir = if apache_path.join("conf").exists() {
        apache_path.join("conf")
    } else {
        return Err("conf directory not found".to_string());
    };

    let httpd_conf = conf_dir.join("httpd.conf");
    if !httpd_conf.exists() {
        return Err("httpd.conf not found".to_string());
    }

    let mut content = fs::read_to_string(&httpd_conf)
        .map_err(|e| format!("Failed to read httpd.conf: {}", e))?;

    let server_root = apache_path.to_string_lossy().replace('\\', "/");

    // Update SRVROOT
    let re = regex::Regex::new(r#"(?m)^Define SRVROOT.*$"#).unwrap();
    content = re.replace(&content, format!(r#"Define SRVROOT "{}""#, server_root)).to_string();

    // Set port to 8082
    let listen_re = regex::Regex::new(r"(?m)^Listen\s+\d+").unwrap();
    content = listen_re.replace(&content, "Listen 8082").to_string();

    // Update ServerName
    let sn_re = regex::Regex::new(r"(?m)^#?ServerName.*$").unwrap();
    content = sn_re.replace(&content, "ServerName localhost:8082").to_string();

    fs::write(&httpd_conf, content).map_err(|e| format!("Failed to write httpd.conf: {}", e))?;

    // Create logs directory
    let logs_dir = apache_path.join("logs");
    fs::create_dir_all(&logs_dir).ok();

    Ok(())
}

// ─── Main ─────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let bin_dir = get_bin_dir();

    match cli.command {
        Commands::Status => cmd_status(&bin_dir),
        Commands::Start { service, all } => cmd_start(&bin_dir, service, all),
        Commands::Stop { service, all } => cmd_stop(&bin_dir, service, all),
        Commands::Restart { service, all } => cmd_restart(&bin_dir, service, all),
        Commands::List => cmd_list(&bin_dir),
        Commands::Sites { json } => cmd_sites(json),
        Commands::Info => cmd_info(&bin_dir),
        Commands::Logs(sub) => match sub {
            LogsCommands::List => cmd_logs_list(&bin_dir),
            LogsCommands::Show { name, lines, follow } => cmd_logs_show(&bin_dir, &name, lines, follow),
            LogsCommands::Clear { name } => cmd_logs_clear(&bin_dir, &name),
        },
        Commands::Db(sub) => match sub {
            DbCommands::List => cmd_db_list(&bin_dir),
            DbCommands::Create { name } => cmd_db_create(&bin_dir, &name),
            DbCommands::Drop { name, yes } => cmd_db_drop(&bin_dir, &name, yes),
            DbCommands::Export { name, output } => cmd_db_export(&bin_dir, &name, output),
            DbCommands::Import { name, file } => cmd_db_import(&bin_dir, &name, &file),
        },
        Commands::Open { target, https } => cmd_open(&target, https),
        Commands::Php(sub) => match sub {
            PhpCommands::List => cmd_php_list(&bin_dir),
            PhpCommands::Ext { version, enable, disable } => cmd_php_ext(&bin_dir, &version, enable, disable),
        },
        Commands::Hosts(sub) => match sub {
            HostsCommands::List => cmd_hosts_list(),
            HostsCommands::Add { domain } => cmd_hosts_add(&domain),
            HostsCommands::Remove { domain } => cmd_hosts_remove(&domain),
        },
        Commands::Composer { args } => cmd_composer(&bin_dir, args),
        Commands::Install { service, version } => cmd_install(&bin_dir, &service, version),
        Commands::Uninstall { service, yes } => cmd_uninstall(&bin_dir, &service, yes),
    }
}
