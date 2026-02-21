//! Orbit MCP Server — Model Context Protocol interface for AI tools
//!
//! Provides a stdio-based MCP server that exposes Orbit's development
//! environment to AI tools like Claude Code, Cursor, and Windsurf.
//!
//! Protocol: JSON-RPC 2.0 over stdio with Content-Length headers
//! Transport: stdin/stdout (protocol), stderr (debug logging)

use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, Write as IoWrite};
use std::path::PathBuf;
use std::process::Command;

// ─── Path Resolution (shared with cli.rs) ────────────────────────

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

// ─── Site Store Types ────────────────────────────────────────────

#[derive(Deserialize)]
struct SiteStore {
    #[allow(dead_code)]
    version: String,
    sites: Vec<SiteMetadata>,
}

#[derive(Deserialize, Clone)]
struct SiteMetadata {
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

fn read_sites_store() -> Result<SiteStore, String> {
    let store_path = get_config_dir().join("sites.json");
    if !store_path.exists() {
        return Ok(SiteStore {
            version: "1".to_string(),
            sites: vec![],
        });
    }
    let content = fs::read_to_string(&store_path)
        .map_err(|e| format!("Failed to read sites.json: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse sites.json: {}", e))
}

// ─── Service Discovery ──────────────────────────────────────────

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
    let output = hidden_command(exe_path).args(args).output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let combined = format!("{}{}", stdout, stderr);

            if pattern.is_empty() {
                let trimmed = combined.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            } else if let Some(pos) = combined.find(pattern) {
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
        let output = hidden_command(&node_exe).arg("--version").output();
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

    // PostgreSQL
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

// ─── Process Management ──────────────────────────────────────────

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
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
        let version_str = name.strip_prefix("php-").unwrap_or("8.4");
        let cleaned: String = version_str.chars().filter(|c| c.is_ascii_digit()).collect();
        let version_num: u32 = cleaned.parse().unwrap_or(84);
        Some(9000 + version_num as u16)
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
            let mut args = Vec::new();
            if let Some(parent) = exe_path.parent() {
                let config = parent.join("redis.conf");
                if config.exists() {
                    args.push(config.to_string_lossy().to_string());
                }
            }
            (exe_path.clone(), args)
        }
        "apache" => (exe_path.clone(), vec![]),
        "mailpit" => (exe_path.clone(), vec![]),
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

// ─── MariaDB Client Discovery ───────────────────────────────────

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
    Err("MariaDB client not found".to_string())
}

// ─── Log File Discovery ─────────────────────────────────────────

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

// ─── Service Name Resolution (aliases) ──────────────────────────

fn resolve_service_name(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "pg" | "postgres" => "postgresql".to_string(),
        "maria" | "mysql" => "mariadb".to_string(),
        "mongo" => "mongodb".to_string(),
        "node" => "nodejs".to_string(),
        "mail" => "mailpit".to_string(),
        other => other.to_string(),
    }
}

// ─── MCP Protocol Layer ─────────────────────────────────────────

fn read_message(reader: &mut impl BufRead) -> Result<Value, String> {
    // Read headers until empty line
    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).map_err(|e| format!("Read error: {}", e))?;
        let header = header.trim().to_string();

        if header.is_empty() {
            break;
        }

        if let Some(len_str) = header.strip_prefix("Content-Length: ") {
            content_length = len_str.parse().map_err(|e| format!("Invalid Content-Length: {}", e))?;
        }
    }

    if content_length == 0 {
        return Err("No Content-Length header".to_string());
    }

    // Read body
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).map_err(|e| format!("Body read error: {}", e))?;

    let body_str = String::from_utf8(body).map_err(|e| format!("UTF-8 error: {}", e))?;
    eprintln!("[orbit-mcp] << {}", body_str);

    serde_json::from_str(&body_str).map_err(|e| format!("JSON parse error: {}", e))
}

fn write_message(msg: &Value) {
    let body = serde_json::to_string(msg).unwrap();
    eprintln!("[orbit-mcp] >> {}", body);
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let stdout = io::stdout();
    let mut out = stdout.lock();
    out.write_all(header.as_bytes()).unwrap();
    out.write_all(body.as_bytes()).unwrap();
    out.flush().unwrap();
}

fn json_rpc_response(id: &Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn json_rpc_error(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

// ─── MCP Handlers ────────────────────────────────────────────────

fn handle_initialize(id: &Value) -> Value {
    json_rpc_response(id, json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "orbit-mcp",
            "version": "1.0.0"
        }
    }))
}

fn handle_tools_list(id: &Value) -> Value {
    let tools = json!([
        {
            "name": "list_services",
            "description": "List all installed services with their status, version, and port. Returns whether each service is currently running or stopped.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "get_service_status",
            "description": "Get detailed status of a specific service including version, port, and running state. Supports aliases: pg/postgres for postgresql, maria/mysql for mariadb, mongo for mongodb, node for nodejs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Service name (e.g., nginx, php-8.4, mariadb, redis, postgresql)"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "start_service",
            "description": "Start a service. Only works for services that have a server process (nginx, php, mariadb, redis, apache, mailpit, postgresql, mongodb).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Service name to start"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "stop_service",
            "description": "Stop a running service.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Service name to stop"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "restart_service",
            "description": "Restart a service (stop then start).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Service name to restart"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "list_sites",
            "description": "List all configured local development sites with their domain, document root, PHP version, SSL status, and web server.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "list_logs",
            "description": "List all available log files with their sizes.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "read_log",
            "description": "Read the last N lines of a log file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Log name (e.g., nginx/access.log, php-8.4/php_errors.log, mariadb/mysql.err)"
                    },
                    "lines": {
                        "type": "number",
                        "description": "Number of lines to read from the end (default: 50)"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "list_databases",
            "description": "List all MariaDB databases. Requires MariaDB to be running.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "create_database",
            "description": "Create a new MariaDB database with utf8mb4 charset. Requires MariaDB to be running.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Database name to create"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "get_system_info",
            "description": "Get Orbit environment information including data directory, installed service count, site count, and system details.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "run_orbit_command",
            "description": "Run an orbit-cli command directly. Use this for operations not covered by other tools (e.g., 'sites --json', 'php list', 'hosts list', 'db export mydb').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The orbit-cli subcommand (e.g., 'status', 'sites', 'php')"
                    },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Additional arguments for the command"
                    }
                },
                "required": ["command"]
            }
        }
    ]);

    json_rpc_response(id, json!({ "tools": tools }))
}

fn handle_tool_call(id: &Value, name: &str, args: &Value) -> Value {
    let result = match name {
        "list_services" => tool_list_services(),
        "get_service_status" => {
            let svc_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_get_service_status(svc_name)
        }
        "start_service" => {
            let svc_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_start_service(svc_name)
        }
        "stop_service" => {
            let svc_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_stop_service(svc_name)
        }
        "restart_service" => {
            let svc_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_restart_service(svc_name)
        }
        "list_sites" => tool_list_sites(),
        "list_logs" => tool_list_logs(),
        "read_log" => {
            let log_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
            tool_read_log(log_name, lines)
        }
        "list_databases" => tool_list_databases(),
        "create_database" => {
            let db_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_create_database(db_name)
        }
        "get_system_info" => tool_get_system_info(),
        "run_orbit_command" => {
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let cmd_args: Vec<String> = args.get("args")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            tool_run_orbit_command(command, &cmd_args)
        }
        _ => Err(format!("Unknown tool: {}", name)),
    };

    match result {
        Ok(content) => json_rpc_response(id, json!({
            "content": [{
                "type": "text",
                "text": content
            }]
        })),
        Err(err) => json_rpc_response(id, json!({
            "content": [{
                "type": "text",
                "text": err
            }],
            "isError": true
        })),
    }
}

// ─── Tool Implementations ────────────────────────────────────────

fn tool_list_services() -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let services = scan_services(&bin_dir);

    if services.is_empty() {
        return Ok("No services installed. Use the Orbit GUI or 'orbit-cli install <service>' to install services.".to_string());
    }

    let mut result = Vec::new();
    for svc in &services {
        let running = is_service_running(&svc.name);
        let port = get_service_port(&svc.name);
        let status = if running { "running" } else { "stopped" };
        let port_str = port.map(|p| format!(":{}", p)).unwrap_or_else(|| "-".to_string());

        result.push(json!({
            "name": svc.name,
            "version": svc.version,
            "status": status,
            "port": port_str,
            "type": svc.service_type
        }));
    }

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_get_service_status(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Service name is required".to_string());
    }

    let resolved = resolve_service_name(name);
    let bin_dir = get_bin_dir();
    let services = scan_services(&bin_dir);

    let service = services.iter().find(|s| {
        s.name == resolved || s.name.starts_with(&resolved)
    });

    match service {
        Some(svc) => {
            let running = is_service_running(&svc.name);
            let port = get_service_port(&svc.name);
            let result = json!({
                "name": svc.name,
                "version": svc.version,
                "status": if running { "running" } else { "stopped" },
                "port": port,
                "path": svc.path,
                "type": svc.service_type
            });
            Ok(serde_json::to_string_pretty(&result).unwrap())
        }
        None => Err(format!("Service '{}' not found. Use list_services to see installed services.", name)),
    }
}

fn tool_start_service(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Service name is required".to_string());
    }

    let resolved = resolve_service_name(name);
    let bin_dir = get_bin_dir();
    let services = scan_services(&bin_dir);

    let service = services.iter().find(|s| {
        s.name == resolved || s.name.starts_with(&resolved)
    });

    match service {
        Some(svc) => {
            if is_service_running(&svc.name) {
                return Ok(format!("{} is already running", svc.name));
            }
            match start_service_process(svc) {
                Ok(pid) => {
                    // Give process a moment to bind
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let port = get_service_port(&svc.name);
                    let port_str = port.map(|p| format!(" on :{}", p)).unwrap_or_default();
                    Ok(format!("{} started{} (PID: {})", svc.name, port_str, pid))
                }
                Err(e) => Err(e),
            }
        }
        None => Err(format!("Service '{}' not found", name)),
    }
}

fn tool_stop_service(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Service name is required".to_string());
    }

    let resolved = resolve_service_name(name);

    if !is_service_running(&resolved) {
        // Try matching with scan
        let bin_dir = get_bin_dir();
        let services = scan_services(&bin_dir);
        let svc_name = services.iter()
            .find(|s| s.name == resolved || s.name.starts_with(&resolved))
            .map(|s| s.name.clone())
            .unwrap_or(resolved.clone());

        if !is_service_running(&svc_name) {
            return Ok(format!("{} is not running", svc_name));
        }
    }

    stop_service_process(&resolved)?;
    Ok(format!("{} stopped", resolved))
}

fn tool_restart_service(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Service name is required".to_string());
    }

    let resolved = resolve_service_name(name);
    let bin_dir = get_bin_dir();
    let services = scan_services(&bin_dir);

    let service = services.iter().find(|s| {
        s.name == resolved || s.name.starts_with(&resolved)
    });

    match service {
        Some(svc) => {
            // Stop if running
            if is_service_running(&svc.name) {
                stop_service_process(&svc.name).ok();
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            match start_service_process(svc) {
                Ok(pid) => {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let port = get_service_port(&svc.name);
                    let port_str = port.map(|p| format!(" on :{}", p)).unwrap_or_default();
                    Ok(format!("{} restarted{} (PID: {})", svc.name, port_str, pid))
                }
                Err(e) => Err(e),
            }
        }
        None => Err(format!("Service '{}' not found", name)),
    }
}

fn tool_list_sites() -> Result<String, String> {
    let store = read_sites_store()?;

    if store.sites.is_empty() {
        return Ok("No sites configured. Use the Orbit GUI to create local development sites.".to_string());
    }

    let mut result = Vec::new();
    for site in &store.sites {
        result.push(json!({
            "domain": site.domain,
            "path": site.path,
            "port": site.port,
            "php_version": site.php_version,
            "ssl_enabled": site.ssl_enabled,
            "web_server": site.web_server,
            "created_at": site.created_at
        }));
    }

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_list_logs() -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let logs = scan_log_files(&bin_dir);

    if logs.is_empty() {
        return Ok("No log files found.".to_string());
    }

    let mut result = Vec::new();
    for log in &logs {
        result.push(json!({
            "name": log.name,
            "size": format_size(log.size),
            "size_bytes": log.size
        }));
    }

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_read_log(name: &str, lines: usize) -> Result<String, String> {
    if name.is_empty() {
        return Err("Log name is required. Use list_logs to see available logs.".to_string());
    }

    let bin_dir = get_bin_dir();
    let logs = scan_log_files(&bin_dir);

    let log = logs.iter().find(|l| l.name == name);

    match log {
        Some(log_file) => {
            let content = fs::read_to_string(&log_file.path)
                .map_err(|e| format!("Failed to read log: {}", e))?;

            let all_lines: Vec<&str> = content.lines().collect();
            let start = if all_lines.len() > lines { all_lines.len() - lines } else { 0 };
            let tail: Vec<&str> = all_lines[start..].to_vec();

            Ok(format!("--- {} (last {} lines, {} total) ---\n{}",
                name,
                tail.len(),
                all_lines.len(),
                tail.join("\n")
            ))
        }
        None => {
            let available: Vec<String> = logs.iter().map(|l| l.name.clone()).collect();
            Err(format!("Log '{}' not found. Available logs: {}", name, available.join(", ")))
        }
    }
}

fn tool_list_databases() -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let client = find_mariadb_client(&bin_dir)?;

    let output = hidden_command(&client)
        .arg("--host=127.0.0.1")
        .arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg("-e").arg("SHOW DATABASES")
        .arg("--batch").arg("--skip-column-names")
        .output()
        .map_err(|e| format!("Failed to run MariaDB client: {}. Is MariaDB running?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("MariaDB error: {}. Is MariaDB running?", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let system_dbs = ["information_schema", "performance_schema", "mysql", "sys"];

    let mut result = Vec::new();
    for db in stdout.lines() {
        let db = db.trim();
        if db.is_empty() { continue; }
        let is_system = system_dbs.contains(&db);
        result.push(json!({
            "name": db,
            "type": if is_system { "system" } else { "user" }
        }));
    }

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_create_database(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Database name is required".to_string());
    }

    // Validate name (alphanumeric + underscore only)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err("Database name can only contain alphanumeric characters, underscores, and hyphens".to_string());
    }

    let bin_dir = get_bin_dir();
    let client = find_mariadb_client(&bin_dir)?;

    let sql = format!("CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci", name);
    let output = hidden_command(&client)
        .arg("--host=127.0.0.1").arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg("-e").arg(&sql)
        .output()
        .map_err(|e| format!("Failed to run MariaDB client: {}", e))?;

    if output.status.success() {
        Ok(format!("Database '{}' created successfully (utf8mb4, utf8mb4_unicode_ci)", name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to create database: {}", stderr.trim()))
    }
}

fn tool_get_system_info() -> Result<String, String> {
    let data_dir = get_orbit_data_dir();
    let bin_dir = get_bin_dir();
    let config_dir = get_config_dir();
    let services = scan_services(&bin_dir);
    let sites = read_sites_store().map(|s| s.sites.len()).unwrap_or(0);
    let logs = scan_log_files(&bin_dir);

    let running_count = services.iter().filter(|s| is_service_running(&s.name)).count();

    let result = json!({
        "orbit_version": "1.0.0",
        "data_directory": data_dir.to_string_lossy(),
        "bin_directory": bin_dir.to_string_lossy(),
        "config_directory": config_dir.to_string_lossy(),
        "services_installed": services.len(),
        "services_running": running_count,
        "sites_configured": sites,
        "log_files": logs.len(),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH
    });

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_run_orbit_command(command: &str, args: &[String]) -> Result<String, String> {
    if command.is_empty() {
        return Err("Command is required".to_string());
    }

    // Find orbit-cli binary (same directory as orbit-mcp)
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Cannot find self: {}", e))?;
    let exe_dir = exe_path.parent()
        .ok_or("Cannot determine executable directory")?;

    let cli_path = exe_dir.join("orbit-cli.exe");
    if !cli_path.exists() {
        // Try finding in PATH
        let cli_path = PathBuf::from("orbit-cli");
        let mut cmd = hidden_command(&cli_path);
        cmd.arg(command);
        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output()
            .map_err(|e| format!("orbit-cli not found: {}. Is it installed?", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        return if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(format!("{}{}", stdout, stderr))
        };
    }

    let mut cmd = hidden_command(&cli_path);
    cmd.arg(command);
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to run orbit-cli: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        Err(format!("{}{}", stdout, stderr))
    }
}

// ─── Entry Point ─────────────────────────────────────────────────

fn main() {
    eprintln!("[orbit-mcp] Orbit MCP Server v1.0.0 starting...");
    eprintln!("[orbit-mcp] Data dir: {}", get_orbit_data_dir().display());

    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());

    loop {
        let msg = match read_message(&mut reader) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("[orbit-mcp] Read error: {}", e);
                break;
            }
        };

        let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let params = msg.get("params").cloned().unwrap_or(json!({}));

        let response = match method {
            "initialize" => Some(handle_initialize(&id)),
            "initialized" => {
                // Notification, no response needed
                eprintln!("[orbit-mcp] Client initialized");
                None
            }
            "tools/list" => Some(handle_tools_list(&id)),
            "tools/call" => {
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let tool_args = params.get("arguments").cloned().unwrap_or(json!({}));
                Some(handle_tool_call(&id, tool_name, &tool_args))
            }
            "notifications/cancelled" => {
                eprintln!("[orbit-mcp] Request cancelled");
                None
            }
            "ping" => Some(json_rpc_response(&id, json!({}))),
            _ => {
                eprintln!("[orbit-mcp] Unknown method: {}", method);
                if !id.is_null() {
                    Some(json_rpc_error(&id, -32601, &format!("Method not found: {}", method)))
                } else {
                    None
                }
            }
        };

        if let Some(resp) = response {
            write_message(&resp);
        }
    }

    eprintln!("[orbit-mcp] Server shutting down");
}
