//! Orbit MCP Server — Model Context Protocol interface for AI tools
//!
//! Provides a stdio-based MCP server that exposes Orbit's development
//! environment to AI tools like Claude Code, Cursor, and Windsurf.
//!
//! Protocol: JSON-RPC 2.0 over stdio with Content-Length headers
//! Transport: stdin/stdout (protocol), stderr (debug logging)

#![recursion_limit = "512"]

use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Serialize)]
struct SiteStore {
    version: String,
    sites: Vec<SiteMetadata>,
}

#[derive(Deserialize, Serialize, Clone)]
struct SiteMetadata {
    domain: String,
    path: String,
    port: u16,
    php_version: Option<String>,
    #[serde(default)]
    php_port: Option<u16>,
    #[serde(default)]
    ssl_enabled: bool,
    #[serde(default)]
    ssl_cert_path: Option<String>,
    #[serde(default)]
    ssl_key_path: Option<String>,
    #[serde(default)]
    template: Option<String>,
    #[serde(default = "default_web_server")]
    web_server: String,
    #[serde(default)]
    dev_port: Option<u16>,
    #[serde(default)]
    dev_command: Option<String>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
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

fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
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
    // Try both 127.0.0.1 and 0.0.0.0 — on Windows, services may bind to either
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

fn require_service(name: &str) -> Result<(), String> {
    let display = match name {
        "mariadb" => "MariaDB",
        "postgresql" => "PostgreSQL",
        "redis" => "Redis",
        "mailpit" => "Mailpit",
        _ => name,
    };
    if !is_service_running(name) {
        return Err(format!("{} is not running. Start it first: start_service {{ \"name\": \"{}\" }}", display, name));
    }
    Ok(())
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

fn find_psql_client(bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let pg_root = bin_dir.join("postgresql");
    let paths = [
        pg_root.join("bin").join("psql.exe"),
        pg_root.join("pgsql").join("bin").join("psql.exe"),
    ];
    for path in paths {
        if path.exists() {
            return Ok(path);
        }
    }
    Err("PostgreSQL client (psql) not found".to_string())
}

fn find_redis_cli(bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let path = bin_dir.join("redis").join("redis-cli.exe");
    if path.exists() {
        return Ok(path);
    }
    Err("Redis CLI not found".to_string())
}

fn find_php_exe(bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let php_root = bin_dir.join("php");
    if php_root.exists() {
        if let Ok(entries) = fs::read_dir(&php_root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let exe = entry.path().join("php.exe");
                    if exe.exists() {
                        return Ok(exe);
                    }
                }
            }
        }
    }
    Err("PHP executable not found".to_string())
}

fn find_composer_phar(bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let path = bin_dir.join("composer").join("composer.phar");
    if path.exists() {
        return Ok(path);
    }
    Err("Composer not found".to_string())
}

fn find_mkcert(bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let path = bin_dir.join("mkcert").join("mkcert.exe");
    if path.exists() {
        return Ok(path);
    }
    Err("mkcert not found".to_string())
}

fn find_nginx_exe(bin_dir: &PathBuf) -> Result<PathBuf, String> {
    let path = bin_dir.join("nginx").join("nginx.exe");
    if path.exists() {
        return Ok(path);
    }
    Err("Nginx not found".to_string())
}

fn nginx_test_and_reload(bin_dir: &PathBuf) -> Result<(), String> {
    let nginx = find_nginx_exe(bin_dir)?;

    // Test config
    let test_output = hidden_command(&nginx)
        .arg("-t")
        .output()
        .map_err(|e| format!("Failed to test nginx config: {}", e))?;

    if !test_output.status.success() {
        let stderr = String::from_utf8_lossy(&test_output.stderr);
        return Err(format!("Nginx config test failed: {}", stderr.trim()));
    }

    // Reload
    let reload_output = hidden_command(&nginx)
        .args(["-s", "reload"])
        .output()
        .map_err(|e| format!("Failed to reload nginx: {}", e))?;

    if !reload_output.status.success() {
        let stderr = String::from_utf8_lossy(&reload_output.stderr);
        return Err(format!("Nginx reload failed: {}", stderr.trim()));
    }

    Ok(())
}

fn backup_file(path: &PathBuf) -> Result<(), String> {
    if path.exists() {
        let bak = path.with_extension(
            format!("{}.bak",
                path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()
            )
        );
        fs::copy(path, &bak)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }
    Ok(())
}

fn write_sites_store(store: &SiteStore) -> Result<(), String> {
    let store_path = get_config_dir().join("sites.json");
    fs::create_dir_all(get_config_dir())
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    let content = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Failed to serialize sites: {}", e))?;
    fs::write(&store_path, content)
        .map_err(|e| format!("Failed to write sites.json: {}", e))
}

fn add_hosts_entry(domain: &str) -> Result<(), String> {
    let hosts_path = PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts");
    let content = fs::read_to_string(&hosts_path)
        .map_err(|e| format!("Failed to read hosts file: {}", e))?;

    let entry = format!("127.0.0.1 {}", domain);
    if content.contains(&entry) {
        return Ok(());
    }

    let new_content = format!("{}\n{}\n", content.trim_end(), entry);
    fs::write(&hosts_path, new_content)
        .map_err(|e| format!("Failed to write hosts file (run as admin?): {}", e))
}

fn remove_hosts_entry(domain: &str) -> Result<(), String> {
    let hosts_path = PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts");
    let content = fs::read_to_string(&hosts_path)
        .map_err(|e| format!("Failed to read hosts file: {}", e))?;

    let entry = format!("127.0.0.1 {}", domain);
    let new_content: String = content.lines()
        .filter(|line| line.trim() != entry)
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(&hosts_path, format!("{}\n", new_content.trim_end()))
        .map_err(|e| format!("Failed to write hosts file (run as admin?): {}", e))
}

fn generate_site_nginx_config(
    domain: &str,
    doc_root: &str,
    php_version: Option<&str>,
    ssl: bool,
    bin_dir: &PathBuf,
) -> String {
    let listen = if ssl {
        format!("    listen 443 ssl;\n    ssl_certificate {ssl_dir}/{domain}.pem;\n    ssl_certificate_key {ssl_dir}/{domain}-key.pem;",
            ssl_dir = bin_dir.join("nginx").join("ssl").display(),
            domain = domain)
    } else {
        "    listen 80;".to_string()
    };

    let php_block = if let Some(ver) = php_version {
        let cleaned: String = ver.chars().filter(|c| c.is_ascii_digit()).collect();
        let port_num: u32 = cleaned.parse().unwrap_or(84);
        let php_port = 9000 + port_num;
        format!(r#"
    location ~ \.php$ {{
        fastcgi_pass 127.0.0.1:{php_port};
        fastcgi_index index.php;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        include fastcgi_params;
    }}"#)
    } else {
        String::new()
    };

    let index = if php_version.is_some() {
        "index.php index.html index.htm"
    } else {
        "index.html index.htm"
    };

    format!(r#"server {{
{listen}
    server_name {domain};
    root {doc_root};
    index {index};

    location / {{
        try_files $uri $uri/ /index.php?$query_string;
    }}
{php_block}

    location ~ /\.ht {{
        deny all;
    }}
}}
"#)
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
        for log_dir_path in &[&mongodb_data as &std::path::Path, bin_dir.join("mongodb").as_path()] {
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
    // Support both Content-Length framing and newline-delimited JSON
    // Claude Code sends raw JSON without Content-Length headers
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(|e| format!("Read error: {}", e))?;
        if bytes == 0 {
            return Err("EOF".to_string());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check if this line starts with Content-Length header (standard MCP framing)
        if trimmed.starts_with("Content-Length:") {
            let len_str = trimmed.strip_prefix("Content-Length:").unwrap().trim();
            let content_length: usize = len_str.parse().map_err(|e| format!("Invalid Content-Length: {}", e))?;

            // Skip remaining headers until empty line
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).map_err(|e| format!("Read error: {}", e))?;
                if header.trim().is_empty() {
                    break;
                }
            }

            // Read body
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body).map_err(|e| format!("Body read error: {}", e))?;
            let body_str = String::from_utf8(body).map_err(|e| format!("UTF-8 error: {}", e))?;
            eprintln!("[orbit-mcp] << {}", body_str);
            return serde_json::from_str(&body_str).map_err(|e| format!("JSON parse error: {}", e));
        }

        // Raw JSON line (newline-delimited mode, used by Claude Code)
        if trimmed.starts_with('{') {
            eprintln!("[orbit-mcp] << {}", trimmed);
            return serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {}", e));
        }

        // Unknown line, skip
        eprintln!("[orbit-mcp] Skipping unknown line: {}", trimmed);
    }
}

fn write_message(msg: &Value) {
    let body = serde_json::to_string(msg).unwrap();
    eprintln!("[orbit-mcp] >> {}", body);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Write as newline-delimited JSON (compatible with Claude Code)
    out.write_all(body.as_bytes()).unwrap();
    out.write_all(b"\n").unwrap();
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
            "version": env!("CARGO_PKG_VERSION")
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
            "description": "Run an orbit-cli command directly. Most operations now have dedicated tools — use this for less common commands (e.g., 'scan', 'open', 'trust-ssl').",
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
        },
        // ─── MariaDB Extended ────────────────────────────
        {
            "name": "list_tables",
            "description": "List all tables in a MariaDB database. Requires MariaDB to be running.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" }
                },
                "required": ["database"]
            }
        },
        {
            "name": "filter_table_names",
            "description": "Filter MariaDB table names using a LIKE pattern. Use '%' as wildcard (e.g., 'wp_%' for WordPress tables).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" },
                    "pattern": { "type": "string", "description": "LIKE pattern (e.g., 'wp_%', '%user%')" }
                },
                "required": ["database", "pattern"]
            }
        },
        {
            "name": "describe_table",
            "description": "Show detailed schema for a MariaDB table including columns, indexes, and foreign keys.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" },
                    "table": { "type": "string", "description": "Table name" }
                },
                "required": ["database", "table"]
            }
        },
        {
            "name": "execute_query",
            "description": "Execute a SQL query on a MariaDB database. Returns results in tab-separated format. Queries are executed directly via CLI — use caution with destructive statements.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" },
                    "query": { "type": "string", "description": "SQL query to execute" }
                },
                "required": ["database", "query"]
            }
        },
        {
            "name": "drop_database",
            "description": "Drop a MariaDB database. This action is irreversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Database name to drop" }
                },
                "required": ["name"]
            }
        },
        // ─── PostgreSQL ──────────────────────────────────
        {
            "name": "pg_list_databases",
            "description": "List all PostgreSQL databases. Requires PostgreSQL to be running.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "pg_list_tables",
            "description": "List all tables in a PostgreSQL database.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" }
                },
                "required": ["database"]
            }
        },
        {
            "name": "pg_filter_table_names",
            "description": "Filter PostgreSQL table names using a LIKE pattern. Use '%' as wildcard.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" },
                    "pattern": { "type": "string", "description": "LIKE pattern (e.g., 'auth_%', '%user%')" }
                },
                "required": ["database", "pattern"]
            }
        },
        {
            "name": "pg_describe_table",
            "description": "Show detailed schema for a PostgreSQL table including columns, indexes, and constraints.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" },
                    "table": { "type": "string", "description": "Table name" }
                },
                "required": ["database", "table"]
            }
        },
        {
            "name": "pg_execute_query",
            "description": "Execute a SQL query on a PostgreSQL database. Queries are executed directly via CLI — use caution with destructive statements.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" },
                    "query": { "type": "string", "description": "SQL query to execute" }
                },
                "required": ["database", "query"]
            }
        },
        // ─── Site Management ─────────────────────────────
        {
            "name": "create_site",
            "description": "Create a new local development site. Adds to sites.json, generates nginx config, adds hosts entry, and reloads nginx.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Domain name (e.g., myapp.test)" },
                    "path": { "type": "string", "description": "Document root path" },
                    "template": { "type": "string", "description": "Site template: static, php, laravel (default: php)" },
                    "php_version": { "type": "string", "description": "PHP version (e.g., 8.4)" },
                    "ssl": { "type": "boolean", "description": "Enable SSL (default: false)" }
                },
                "required": ["domain", "path"]
            }
        },
        {
            "name": "delete_site",
            "description": "Delete a local development site. Removes from sites.json, deletes nginx config, removes hosts entry, and reloads nginx.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Domain to delete" }
                },
                "required": ["domain"]
            }
        },
        {
            "name": "get_site_config",
            "description": "Read the nginx config file for a specific site.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Site domain" }
                },
                "required": ["domain"]
            }
        },
        // ─── SSL ─────────────────────────────────────────
        {
            "name": "generate_ssl",
            "description": "Generate a self-signed SSL certificate for a domain using mkcert.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Domain name (e.g., myapp.test)" }
                },
                "required": ["domain"]
            }
        },
        {
            "name": "list_ssl_certs",
            "description": "List all SSL certificates in the nginx ssl directory.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        // ─── PHP Config ──────────────────────────────────
        {
            "name": "list_php_extensions",
            "description": "List PHP extensions and their enabled/disabled status for a specific PHP version.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "version": { "type": "string", "description": "PHP version (e.g., 8.4)" }
                },
                "required": ["version"]
            }
        },
        {
            "name": "toggle_php_extension",
            "description": "Enable or disable a PHP extension by toggling the semicolon prefix in php.ini.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "version": { "type": "string", "description": "PHP version (e.g., 8.4)" },
                    "extension": { "type": "string", "description": "Extension name (e.g., gd, curl, pdo_mysql)" },
                    "enabled": { "type": "boolean", "description": "true to enable, false to disable" }
                },
                "required": ["version", "extension", "enabled"]
            }
        },
        {
            "name": "get_php_config",
            "description": "Get key PHP configuration values (memory_limit, upload_max_filesize, etc.) from php.ini.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "version": { "type": "string", "description": "PHP version (e.g., 8.4)" }
                },
                "required": ["version"]
            }
        },
        {
            "name": "set_php_config",
            "description": "Set a PHP configuration value in php.ini.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "version": { "type": "string", "description": "PHP version (e.g., 8.4)" },
                    "key": { "type": "string", "description": "Config key (e.g., memory_limit, upload_max_filesize)" },
                    "value": { "type": "string", "description": "Config value (e.g., 256M, 64M)" }
                },
                "required": ["version", "key", "value"]
            }
        },
        // ─── Composer ────────────────────────────────────
        {
            "name": "composer_require",
            "description": "Install a Composer package in a project.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_path": { "type": "string", "description": "Path to the project directory" },
                    "package": { "type": "string", "description": "Package name (e.g., laravel/framework)" },
                    "dev": { "type": "boolean", "description": "Install as dev dependency (default: false)" }
                },
                "required": ["project_path", "package"]
            }
        },
        {
            "name": "composer_install",
            "description": "Run composer install in a project directory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_path": { "type": "string", "description": "Path to the project directory" }
                },
                "required": ["project_path"]
            }
        },
        {
            "name": "composer_run",
            "description": "Run a Composer script defined in composer.json.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_path": { "type": "string", "description": "Path to the project directory" },
                    "script": { "type": "string", "description": "Script name to run" }
                },
                "required": ["project_path", "script"]
            }
        },
        // ─── Redis ───────────────────────────────────────
        {
            "name": "redis_command",
            "description": "Execute a Redis command via redis-cli.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Redis command (e.g., PING, GET key, SET key value, KEYS *)" }
                },
                "required": ["command"]
            }
        },
        {
            "name": "redis_info",
            "description": "Get Redis server information (INFO command).",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        // ─── Mailpit ────────────────────────────────────
        {
            "name": "list_emails",
            "description": "List emails captured by Mailpit. Requires Mailpit to be running on port 8025.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Max emails to return (default: 50)" }
                },
                "required": []
            }
        },
        {
            "name": "get_email",
            "description": "Get a specific email from Mailpit by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Email message ID" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "delete_emails",
            "description": "Delete all emails in Mailpit.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        // ─── Config Files ────────────────────────────────
        {
            "name": "read_config",
            "description": "Read a service configuration file. Types: nginx, apache, mariadb, php.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "description": "Config type: nginx, apache, mariadb, php" },
                    "php_version": { "type": "string", "description": "PHP version (required when type is php)" }
                },
                "required": ["type"]
            }
        },
        {
            "name": "write_config",
            "description": "Write a service configuration file. Creates a .bak backup first. Types: nginx, apache, mariadb, php.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "description": "Config type: nginx, apache, mariadb, php" },
                    "content": { "type": "string", "description": "New config file content" },
                    "php_version": { "type": "string", "description": "PHP version (required when type is php)" }
                },
                "required": ["type", "content"]
            }
        },
        {
            "name": "read_site_config",
            "description": "Read the nginx or apache vhost config for a specific site domain.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Site domain" }
                },
                "required": ["domain"]
            }
        },
        {
            "name": "write_site_config",
            "description": "Write a site's nginx config. Runs nginx -t to validate, rolls back on failure.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Site domain" },
                    "content": { "type": "string", "description": "New nginx config content" }
                },
                "required": ["domain", "content"]
            }
        },
        // ─── Batch Operations ────────────────────────────
        {
            "name": "start_all_services",
            "description": "Start all installed server services (nginx, php, mariadb, redis, apache, mailpit, postgresql, mongodb).",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "stop_all_services",
            "description": "Stop all running server services.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        // ─── Hosts File ──────────────────────────────────
        {
            "name": "hosts_list",
            "description": "List all entries in the system hosts file. Shows IP and domain mappings.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "hosts_add",
            "description": "Add a domain to the hosts file pointing to 127.0.0.1. Requires admin/elevated privileges.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Domain to add (e.g., myapp.test)" }
                },
                "required": ["domain"]
            }
        },
        {
            "name": "hosts_remove",
            "description": "Remove a domain from the hosts file. Requires admin/elevated privileges.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Domain to remove" }
                },
                "required": ["domain"]
            }
        },
        // ─── Database Export/Import ──────────────────────
        {
            "name": "db_export",
            "description": "Export a MariaDB database to a SQL file using mysqldump. Returns the output file path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name to export" },
                    "output": { "type": "string", "description": "Output file path (default: <database>.sql in current dir)" }
                },
                "required": ["database"]
            }
        },
        {
            "name": "db_import",
            "description": "Import a SQL file into a MariaDB database.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Target database name" },
                    "file": { "type": "string", "description": "Path to the SQL file to import" }
                },
                "required": ["database", "file"]
            }
        },
        // ─── Log Management ─────────────────────────────
        {
            "name": "clear_log",
            "description": "Clear a log file (truncate to empty).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Log name (e.g., nginx/access.log, php-8.4/php_errors.log)" }
                },
                "required": ["name"]
            }
        },
        // ─── Service Install/Uninstall ──────────────────
        {
            "name": "install_service",
            "description": "Install a service from the Orbit registry. Downloads and extracts the service binary. Supported: nginx, php, mariadb, postgresql, mongodb, redis, nodejs, python, bun, deno, go, apache, mailpit, composer, rust.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service": { "type": "string", "description": "Service to install (e.g., nginx, php, mariadb, redis)" },
                    "version": { "type": "string", "description": "Version to install (e.g., 8.4 for PHP). Uses latest if omitted." }
                },
                "required": ["service"]
            }
        },
        {
            "name": "uninstall_service",
            "description": "Uninstall a service by removing its directory. Stops the service first if running.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service": { "type": "string", "description": "Service to uninstall (e.g., nginx, mariadb, php-8.4)" }
                },
                "required": ["service"]
            }
        },
        // ─── AI Diagnostics ────────────────────────────────
        {
            "name": "diagnose_service",
            "description": "Run a comprehensive health check on a service. Checks binary existence, port status, process state, config validation, and error logs. Returns status (healthy/degraded/down/not_installed), issues, suggestions, and details.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Service name (e.g., nginx, php-8.4, mariadb, redis, postgresql)" }
                },
                "required": ["name"]
            }
        },
        {
            "name": "diagnose_site",
            "description": "Run a health check on a local development site. Checks site config, web server status, PHP version, hosts entry, SSL certs, and reachability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Site domain (e.g., myapp.test)" }
                },
                "required": ["domain"]
            }
        },
        {
            "name": "analyze_logs",
            "description": "Analyze log files for error patterns. Groups errors by frequency, provides known solutions for common issues (502 Bad Gateway, Permission denied, PHP Fatal, etc.).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service": { "type": "string", "description": "Service name to analyze logs for. If omitted, analyzes all available logs." },
                    "lines": { "type": "number", "description": "Number of lines to analyze from each log (default: 200)" },
                    "severity": { "type": "string", "description": "Minimum severity: error, warning, all (default: error)" }
                },
                "required": []
            }
        },
        {
            "name": "get_health_report",
            "description": "Generate a comprehensive system health report. Checks all services, port conflicts, disk usage, site issues, large log files, and calculates a health score (0-100).",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        // ─── Blueprint System ──────────────────────────────
        {
            "name": "list_blueprints",
            "description": "List all available project blueprints. Blueprints define a complete project setup: required services, site template, and scaffold commands.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "get_blueprint",
            "description": "Get detailed information about a specific blueprint including required services, template, scaffold commands, and PHP extensions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Blueprint name (e.g., laravel-vite, nextjs-fullstack, django)" }
                },
                "required": ["name"]
            }
        },
        {
            "name": "create_from_blueprint",
            "description": "Create a complete project from a blueprint. Validates required services, starts them, creates a site with the correct template, runs scaffold commands, adds hosts entry, generates SSL if needed, and writes .env file. One-click project setup.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "blueprint": { "type": "string", "description": "Blueprint name (e.g., laravel-vite, django, nextjs-fullstack)" },
                    "domain": { "type": "string", "description": "Domain name (e.g., myapp.test)" },
                    "path": { "type": "string", "description": "Project directory path" },
                    "php_version": { "type": "string", "description": "PHP version override (default: 8.4)" }
                },
                "required": ["blueprint", "domain", "path"]
            }
        },
        {
            "name": "start_site_app",
            "description": "Start a site's development server using its configured dev_command. Only works for sites that have a dev_command set (typically from blueprint creation). The process runs in the background.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Site domain (e.g., myapp.test)" }
                },
                "required": ["domain"]
            }
        },
        {
            "name": "stop_site_app",
            "description": "Stop a site's development server that was previously started with start_site_app.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "Site domain (e.g., myapp.test)" }
                },
                "required": ["domain"]
            }
        },
        // ─── MongoDB ──────────────────────────────────────
        {
            "name": "mongo_list_databases",
            "description": "List all MongoDB databases. Requires MongoDB to be running.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        },
        {
            "name": "mongo_list_collections",
            "description": "List all collections in a MongoDB database.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" }
                },
                "required": ["database"]
            }
        },
        {
            "name": "mongo_execute",
            "description": "Execute a JavaScript command in a MongoDB database via mongosh. Queries are executed directly — use caution with destructive operations.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database": { "type": "string", "description": "Database name" },
                    "command": { "type": "string", "description": "JavaScript command to execute (e.g., 'db.users.find({})' or 'db.stats()')" }
                },
                "required": ["database", "command"]
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
        // MariaDB extended
        "list_tables" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            tool_list_tables(db)
        }
        "filter_table_names" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("%");
            tool_filter_table_names(db, pattern)
        }
        "describe_table" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let table = args.get("table").and_then(|v| v.as_str()).unwrap_or("");
            tool_describe_table(db, table)
        }
        "execute_query" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            tool_execute_query(db, query)
        }
        "drop_database" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_drop_database(name)
        }
        // PostgreSQL
        "pg_list_databases" => tool_pg_list_databases(),
        "pg_list_tables" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            tool_pg_list_tables(db)
        }
        "pg_filter_table_names" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("%");
            tool_pg_filter_table_names(db, pattern)
        }
        "pg_describe_table" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let table = args.get("table").and_then(|v| v.as_str()).unwrap_or("");
            tool_pg_describe_table(db, table)
        }
        "pg_execute_query" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            tool_pg_execute_query(db, query)
        }
        // Site management
        "create_site" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let template = args.get("template").and_then(|v| v.as_str());
            let php_version = args.get("php_version").and_then(|v| v.as_str());
            let ssl = args.get("ssl").and_then(|v| v.as_bool()).unwrap_or(false);
            tool_create_site(domain, path, template, php_version, ssl)
        }
        "delete_site" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_delete_site(domain)
        }
        "get_site_config" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_get_site_config(domain)
        }
        // SSL
        "generate_ssl" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_generate_ssl(domain)
        }
        "list_ssl_certs" => tool_list_ssl_certs(),
        // PHP config
        "list_php_extensions" => {
            let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("");
            tool_list_php_extensions(version)
        }
        "toggle_php_extension" => {
            let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("");
            let ext = args.get("extension").and_then(|v| v.as_str()).unwrap_or("");
            let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            tool_toggle_php_extension(version, ext, enabled)
        }
        "get_php_config" => {
            let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("");
            tool_get_php_config(version)
        }
        "set_php_config" => {
            let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("");
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
            tool_set_php_config(version, key, value)
        }
        // Composer
        "composer_require" => {
            let project = args.get("project_path").and_then(|v| v.as_str()).unwrap_or("");
            let package = args.get("package").and_then(|v| v.as_str()).unwrap_or("");
            let dev = args.get("dev").and_then(|v| v.as_bool()).unwrap_or(false);
            tool_composer_require(project, package, dev)
        }
        "composer_install" => {
            let project = args.get("project_path").and_then(|v| v.as_str()).unwrap_or("");
            tool_composer_install(project)
        }
        "composer_run" => {
            let project = args.get("project_path").and_then(|v| v.as_str()).unwrap_or("");
            let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");
            tool_composer_run(project, script)
        }
        // Redis
        "redis_command" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            tool_redis_command(cmd)
        }
        "redis_info" => tool_redis_info(),
        // Mailpit
        "list_emails" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
            tool_list_emails(limit)
        }
        "get_email" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            tool_get_email(id)
        }
        "delete_emails" => tool_delete_emails(),
        // Config files
        "read_config" => {
            let config_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let php_version = args.get("php_version").and_then(|v| v.as_str());
            tool_read_config(config_type, php_version)
        }
        "write_config" => {
            let config_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let php_version = args.get("php_version").and_then(|v| v.as_str());
            tool_write_config(config_type, content, php_version)
        }
        "read_site_config" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_read_site_config(domain)
        }
        "write_site_config" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            tool_write_site_config(domain, content)
        }
        // Batch operations
        "start_all_services" => tool_start_all_services(),
        "stop_all_services" => tool_stop_all_services(),
        // Hosts
        "hosts_list" => tool_hosts_list(),
        "hosts_add" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_hosts_add(domain)
        }
        "hosts_remove" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_hosts_remove(domain)
        }
        // DB export/import
        "db_export" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let output = args.get("output").and_then(|v| v.as_str());
            tool_db_export(db, output)
        }
        "db_import" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let file = args.get("file").and_then(|v| v.as_str()).unwrap_or("");
            tool_db_import(db, file)
        }
        // Log management
        "clear_log" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_clear_log(name)
        }
        // Service install/uninstall
        "install_service" => {
            let service = args.get("service").and_then(|v| v.as_str()).unwrap_or("");
            let version = args.get("version").and_then(|v| v.as_str());
            tool_install_service(service, version)
        }
        "uninstall_service" => {
            let service = args.get("service").and_then(|v| v.as_str()).unwrap_or("");
            tool_uninstall_service(service)
        }
        // Diagnostics
        "diagnose_service" => {
            let svc_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_diagnose_service(svc_name)
        }
        "diagnose_site" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_diagnose_site(domain)
        }
        "analyze_logs" => {
            let service = args.get("service").and_then(|v| v.as_str());
            let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
            let severity = args.get("severity").and_then(|v| v.as_str()).unwrap_or("error");
            tool_analyze_logs(service, lines, severity)
        }
        "get_health_report" => tool_get_health_report(),
        // Blueprints
        "list_blueprints" => tool_list_blueprints(),
        "get_blueprint" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            tool_get_blueprint(name)
        }
        "create_from_blueprint" => {
            let blueprint = args.get("blueprint").and_then(|v| v.as_str()).unwrap_or("");
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let php_version = args.get("php_version").and_then(|v| v.as_str());
            tool_create_from_blueprint(blueprint, domain, path, php_version)
        }
        // Site app process management
        "start_site_app" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_start_site_app(domain)
        }
        "stop_site_app" => {
            let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            tool_stop_site_app(domain)
        }
        // MongoDB
        "mongo_list_databases" => tool_mongo_list_databases(),
        "mongo_list_collections" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            tool_mongo_list_collections(db)
        }
        "mongo_execute" => {
            let db = args.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            tool_mongo_execute(db, cmd)
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
    require_service("mariadb")?;
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
    require_service("mariadb")?;
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
        "orbit_version": env!("CARGO_PKG_VERSION"),
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

// ─── MariaDB Extended Tools ──────────────────────────────────────

fn run_mariadb_query(sql: &str) -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let client = find_mariadb_client(&bin_dir)?;

    let output = hidden_command(&client)
        .arg("--host=127.0.0.1").arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg("--batch")
        .arg("-e").arg(sql)
        .output()
        .map_err(|e| format!("Failed to run MariaDB client: {}. Is MariaDB running?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("MariaDB error: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn tool_list_tables(database: &str) -> Result<String, String> {
    require_service("mariadb")?;
    if database.is_empty() {
        return Err("Database name is required".to_string());
    }
    let sql = format!("SHOW TABLES FROM `{}`", database);
    let output = run_mariadb_query(&sql)?;

    let tables: Vec<&str> = output.lines().skip(1).collect(); // skip header
    let result: Vec<Value> = tables.iter()
        .filter(|t| !t.trim().is_empty())
        .map(|t| json!({ "table": t.trim() }))
        .collect();

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_filter_table_names(database: &str, pattern: &str) -> Result<String, String> {
    require_service("mariadb")?;
    if database.is_empty() || pattern.is_empty() {
        return Err("Database and pattern are required".to_string());
    }
    let sql = format!("SHOW TABLES FROM `{}` LIKE '{}'", database, pattern.replace('\'', "\\'"));
    let output = run_mariadb_query(&sql)?;

    let tables: Vec<&str> = output.lines().skip(1).collect();
    let result: Vec<Value> = tables.iter()
        .filter(|t| !t.trim().is_empty())
        .map(|t| json!({ "table": t.trim() }))
        .collect();

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_describe_table(database: &str, table: &str) -> Result<String, String> {
    require_service("mariadb")?;
    if database.is_empty() || table.is_empty() {
        return Err("Database and table name are required".to_string());
    }

    // 1. Column definitions
    let columns_sql = format!("SHOW COLUMNS FROM `{}`.`{}`", database, table);
    let columns = run_mariadb_query(&columns_sql)?;

    // 2. Indexes
    let indexes_sql = format!("SHOW INDEX FROM `{}`.`{}`", database, table);
    let indexes = run_mariadb_query(&indexes_sql).unwrap_or_else(|_| "No indexes found".to_string());

    // 3. Foreign keys
    let fk_sql = format!(
        "SELECT CONSTRAINT_NAME, COLUMN_NAME, REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME \
         FROM information_schema.KEY_COLUMN_USAGE \
         WHERE TABLE_SCHEMA='{}' AND TABLE_NAME='{}' AND REFERENCED_TABLE_NAME IS NOT NULL",
        database.replace('\'', "\\'"), table.replace('\'', "\\'")
    );
    let fks = run_mariadb_query(&fk_sql).unwrap_or_else(|_| "No foreign keys".to_string());

    Ok(format!("=== Columns ===\n{}\n\n=== Indexes ===\n{}\n\n=== Foreign Keys ===\n{}", columns, indexes, fks))
}

fn tool_execute_query(database: &str, query: &str) -> Result<String, String> {
    require_service("mariadb")?;
    if database.is_empty() || query.is_empty() {
        return Err("Database and query are required".to_string());
    }
    let sql = format!("USE `{}`; {}", database, query);
    run_mariadb_query(&sql)
}

fn tool_drop_database(name: &str) -> Result<String, String> {
    require_service("mariadb")?;
    if name.is_empty() {
        return Err("Database name is required".to_string());
    }

    let system_dbs = ["information_schema", "performance_schema", "mysql", "sys"];
    if system_dbs.contains(&name) {
        return Err(format!("Cannot drop system database: {}", name));
    }

    let sql = format!("DROP DATABASE IF EXISTS `{}`", name);
    run_mariadb_query(&sql)?;
    Ok(format!("Database '{}' dropped successfully", name))
}

// ─── PostgreSQL Tools ────────────────────────────────────────────

fn run_psql_query(database: Option<&str>, command: &str) -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let psql = find_psql_client(&bin_dir)?;

    let mut cmd = hidden_command(&psql);
    cmd.arg("-U").arg("postgres")
       .arg("-h").arg("127.0.0.1")
       .arg("-p").arg("5432");

    if let Some(db) = database {
        cmd.arg("-d").arg(db);
    }

    cmd.arg("-c").arg(command);

    // Set PGPASSWORD if needed
    cmd.env("PGPASSWORD", "postgres");

    let output = cmd.output()
        .map_err(|e| format!("Failed to run psql: {}. Is PostgreSQL running?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PostgreSQL error: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn tool_pg_list_databases() -> Result<String, String> {
    require_service("postgresql")?;
    let bin_dir = get_bin_dir();
    let psql = find_psql_client(&bin_dir)?;

    let output = hidden_command(&psql)
        .arg("-U").arg("postgres")
        .arg("-h").arg("127.0.0.1")
        .arg("-p").arg("5432")
        .arg("-l").arg("--csv")
        .env("PGPASSWORD", "postgres")
        .output()
        .map_err(|e| format!("Failed to run psql: {}. Is PostgreSQL running?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PostgreSQL error: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn tool_pg_list_tables(database: &str) -> Result<String, String> {
    require_service("postgresql")?;
    if database.is_empty() {
        return Err("Database name is required".to_string());
    }
    run_psql_query(Some(database), "\\dt")
}

fn tool_pg_filter_table_names(database: &str, pattern: &str) -> Result<String, String> {
    require_service("postgresql")?;
    if database.is_empty() || pattern.is_empty() {
        return Err("Database and pattern are required".to_string());
    }
    let sql = format!(
        "SELECT tablename FROM pg_tables WHERE schemaname='public' AND tablename LIKE '{}'",
        pattern.replace('\'', "''")
    );
    run_psql_query(Some(database), &sql)
}

fn tool_pg_describe_table(database: &str, table: &str) -> Result<String, String> {
    require_service("postgresql")?;
    if database.is_empty() || table.is_empty() {
        return Err("Database and table name are required".to_string());
    }

    // 1. Detailed table description (columns, types, defaults)
    let columns = run_psql_query(Some(database), &format!("\\d+ {}", table))?;

    // 2. Constraints (PK, FK, unique, check)
    let constraints_sql = format!(
        "SELECT conname, contype, pg_get_constraintdef(oid) \
         FROM pg_constraint \
         WHERE conrelid = '{}'::regclass",
        table.replace('\'', "''")
    );
    let constraints = run_psql_query(Some(database), &constraints_sql)
        .unwrap_or_else(|_| "No constraints found".to_string());

    Ok(format!("=== Table Structure ===\n{}\n\n=== Constraints ===\n{}", columns, constraints))
}

fn tool_pg_execute_query(database: &str, query: &str) -> Result<String, String> {
    require_service("postgresql")?;
    if database.is_empty() || query.is_empty() {
        return Err("Database and query are required".to_string());
    }
    run_psql_query(Some(database), query)
}

// ─── MongoDB Tools ───────────────────────────────────────────────

fn find_mongosh_client(bin_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    // Try mongosh first (modern client)
    let paths = [
        bin_dir.join("mongodb").join("bin").join("mongosh.exe"),
        bin_dir.join("mongodb").join("mongosh.exe"),
        bin_dir.join("mongodb").join("bin").join("mongo.exe"),
    ];

    for p in &paths {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    // Try PATH
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = hidden_command("where").arg("mongosh").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = path.lines().next() {
                    return Ok(std::path::PathBuf::from(line.trim()));
                }
            }
        }
        if let Ok(output) = hidden_command("where").arg("mongo").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = path.lines().next() {
                    return Ok(std::path::PathBuf::from(line.trim()));
                }
            }
        }
    }

    Err("mongosh/mongo client not found. Is MongoDB installed?".to_string())
}

fn run_mongosh_command(database: Option<&str>, js_command: &str) -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let mongosh = find_mongosh_client(&bin_dir)?;

    let db = database.unwrap_or("admin");

    let output = hidden_command(&mongosh)
        .arg("--host").arg("127.0.0.1")
        .arg("--port").arg("27017")
        .arg("--quiet")
        .arg(db)
        .arg("--eval").arg(js_command)
        .output()
        .map_err(|e| format!("Failed to run mongosh: {}. Is MongoDB running?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("MongoDB error: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn tool_mongo_list_databases() -> Result<String, String> {
    require_service("mongodb")?;
    let result = run_mongosh_command(Some("admin"), "JSON.stringify(db.adminCommand('listDatabases').databases)")?;
    Ok(result)
}

fn tool_mongo_list_collections(database: &str) -> Result<String, String> {
    require_service("mongodb")?;
    if database.is_empty() {
        return Err("Database name is required".to_string());
    }
    let result = run_mongosh_command(Some(database), "JSON.stringify(db.getCollectionNames())")?;
    Ok(result)
}

fn tool_mongo_execute(database: &str, command: &str) -> Result<String, String> {
    require_service("mongodb")?;
    if database.is_empty() || command.is_empty() {
        return Err("Database and command are required".to_string());
    }
    run_mongosh_command(Some(database), command)
}

// ─── Site Management Tools ───────────────────────────────────────

fn tool_create_site(
    domain: &str,
    path: &str,
    template: Option<&str>,
    php_version: Option<&str>,
    ssl: bool,
) -> Result<String, String> {
    if domain.is_empty() || path.is_empty() {
        return Err("Domain and path are required".to_string());
    }

    let bin_dir = get_bin_dir();
    let mut store = read_sites_store()?;

    // Check if domain already exists
    if store.sites.iter().any(|s| s.domain == domain) {
        return Err(format!("Site '{}' already exists", domain));
    }

    let _template = template.unwrap_or("php");
    let php_ver = match _template {
        "static" => None,
        _ => Some(php_version.unwrap_or("8.4").to_string()),
    };

    // Add to sites.json
    let now = chrono_now();
    let site = SiteMetadata {
        domain: domain.to_string(),
        path: path.to_string(),
        port: if ssl { 443 } else { 80 },
        php_version: php_ver.clone(),
        php_port: None,
        ssl_enabled: ssl,
        ssl_cert_path: None,
        ssl_key_path: None,
        template: template.map(|t| t.to_string()),
        web_server: "nginx".to_string(),
        dev_port: None,
        dev_command: None,
        created_at: now.clone(),
        updated_at: now,
    };
    store.sites.push(site);
    write_sites_store(&store)?;

    // Generate nginx config
    let config = generate_site_nginx_config(
        domain,
        path,
        php_ver.as_deref(),
        ssl,
        &bin_dir,
    );

    let sites_dir = bin_dir.join("nginx").join("conf").join("sites-enabled");
    fs::create_dir_all(&sites_dir)
        .map_err(|e| format!("Failed to create sites-enabled dir: {}", e))?;
    let conf_path = sites_dir.join(format!("{}.conf", domain));
    fs::write(&conf_path, &config)
        .map_err(|e| format!("Failed to write nginx config: {}", e))?;

    // Add hosts entry
    add_hosts_entry(domain).ok(); // Don't fail if hosts write fails

    // Reload nginx if running
    if is_service_running("nginx") {
        nginx_test_and_reload(&bin_dir).ok();
    }

    Ok(format!("Site '{}' created successfully\nDocument root: {}\nNginx config: {}",
        domain, path, conf_path.display()))
}

fn tool_delete_site(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let mut store = read_sites_store()?;

    let initial_len = store.sites.len();
    store.sites.retain(|s| s.domain != domain);
    if store.sites.len() == initial_len {
        return Err(format!("Site '{}' not found", domain));
    }

    write_sites_store(&store)?;

    // Remove nginx config
    let conf_path = bin_dir.join("nginx").join("conf").join("sites-enabled").join(format!("{}.conf", domain));
    if conf_path.exists() {
        fs::remove_file(&conf_path).ok();
    }

    // Remove hosts entry
    remove_hosts_entry(domain).ok();

    // Reload nginx if running
    if is_service_running("nginx") {
        nginx_test_and_reload(&bin_dir).ok();
    }

    Ok(format!("Site '{}' deleted successfully", domain))
}

fn tool_get_site_config(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let conf_path = bin_dir.join("nginx").join("conf").join("sites-enabled").join(format!("{}.conf", domain));

    if !conf_path.exists() {
        return Err(format!("No nginx config found for '{}'", domain));
    }

    fs::read_to_string(&conf_path)
        .map_err(|e| format!("Failed to read config: {}", e))
}

// ─── SSL Tools ───────────────────────────────────────────────────

fn tool_generate_ssl(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let mkcert = find_mkcert(&bin_dir)?;

    let ssl_dir = bin_dir.join("nginx").join("ssl");
    fs::create_dir_all(&ssl_dir)
        .map_err(|e| format!("Failed to create ssl dir: {}", e))?;

    let cert_file = ssl_dir.join(format!("{}.pem", domain));
    let key_file = ssl_dir.join(format!("{}-key.pem", domain));

    let output = hidden_command(&mkcert)
        .arg("-cert-file").arg(&cert_file)
        .arg("-key-file").arg(&key_file)
        .arg(domain)
        .arg(format!("*.{}", domain))
        .arg("localhost")
        .arg("127.0.0.1")
        .arg("::1")
        .output()
        .map_err(|e| format!("Failed to run mkcert: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("mkcert error: {}", stderr.trim()));
    }

    Ok(format!("SSL certificate generated:\n  cert: {}\n  key: {}",
        cert_file.display(), key_file.display()))
}

fn tool_list_ssl_certs() -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let ssl_dir = bin_dir.join("nginx").join("ssl");

    if !ssl_dir.exists() {
        return Ok("No SSL certificates found. SSL directory does not exist.".to_string());
    }

    let mut certs = Vec::new();
    if let Ok(entries) = fs::read_dir(&ssl_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if fname.ends_with(".pem") && !fname.ends_with("-key.pem") {
                let domain = fname.trim_end_matches(".pem");
                let has_key = ssl_dir.join(format!("{}-key.pem", domain)).exists();
                certs.push(json!({
                    "domain": domain,
                    "cert": path.display().to_string(),
                    "has_key": has_key
                }));
            }
        }
    }

    if certs.is_empty() {
        return Ok("No SSL certificates found.".to_string());
    }

    Ok(serde_json::to_string_pretty(&certs).unwrap())
}

// ─── PHP Config Tools ────────────────────────────────────────────

fn get_php_ini_path(bin_dir: &PathBuf, version: &str) -> Result<PathBuf, String> {
    let ini_path = bin_dir.join("php").join(version).join("php.ini");
    if ini_path.exists() {
        Ok(ini_path)
    } else {
        Err(format!("php.ini not found for PHP {}", version))
    }
}

fn tool_list_php_extensions(version: &str) -> Result<String, String> {
    if version.is_empty() {
        return Err("PHP version is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let ini_path = get_php_ini_path(&bin_dir, version)?;
    let content = fs::read_to_string(&ini_path)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    let mut extensions = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("extension=") {
            let ext = trimmed.strip_prefix("extension=").unwrap().trim();
            extensions.push(json!({ "name": ext, "enabled": true }));
        } else if trimmed.starts_with(";extension=") {
            let ext = trimmed.strip_prefix(";extension=").unwrap().trim();
            extensions.push(json!({ "name": ext, "enabled": false }));
        }
    }

    // Also scan ext/ directory for available extensions
    let ext_dir = bin_dir.join("php").join(version).join("ext");
    if ext_dir.exists() {
        if let Ok(entries) = fs::read_dir(&ext_dir) {
            let known_names: Vec<String> = extensions.iter()
                .filter_map(|e| e.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect();

            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with("php_") && fname.ends_with(".dll") {
                    let ext_name = fname.trim_start_matches("php_").trim_end_matches(".dll");
                    if !known_names.contains(&ext_name.to_string()) {
                        extensions.push(json!({ "name": ext_name, "enabled": false, "available": true }));
                    }
                }
            }
        }
    }

    Ok(serde_json::to_string_pretty(&extensions).unwrap())
}

fn tool_toggle_php_extension(version: &str, extension: &str, enabled: bool) -> Result<String, String> {
    if version.is_empty() || extension.is_empty() {
        return Err("Version and extension are required".to_string());
    }

    let bin_dir = get_bin_dir();
    let ini_path = get_php_ini_path(&bin_dir, version)?;
    backup_file(&ini_path)?;

    let content = fs::read_to_string(&ini_path)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    let enabled_line = format!("extension={}", extension);
    let disabled_line = format!(";extension={}", extension);

    let mut found = false;
    let new_content: String = content.lines().map(|line| {
        let trimmed = line.trim();
        if enabled && trimmed == disabled_line {
            found = true;
            enabled_line.clone()
        } else if !enabled && trimmed == enabled_line {
            found = true;
            disabled_line.clone()
        } else {
            line.to_string()
        }
    }).collect::<Vec<_>>().join("\n");

    if !found && enabled {
        // Add the extension line if not found
        let new_content = format!("{}\n{}\n", new_content.trim_end(), enabled_line);
        fs::write(&ini_path, new_content)
            .map_err(|e| format!("Failed to write php.ini: {}", e))?;
        return Ok(format!("Extension '{}' added and enabled for PHP {}", extension, version));
    }

    if !found {
        return Err(format!("Extension '{}' not found in php.ini for PHP {}", extension, version));
    }

    fs::write(&ini_path, new_content)
        .map_err(|e| format!("Failed to write php.ini: {}", e))?;

    let action = if enabled { "enabled" } else { "disabled" };
    Ok(format!("Extension '{}' {} for PHP {}", extension, action, version))
}

fn tool_get_php_config(version: &str) -> Result<String, String> {
    if version.is_empty() {
        return Err("PHP version is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let ini_path = get_php_ini_path(&bin_dir, version)?;
    let content = fs::read_to_string(&ini_path)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    let keys = [
        "memory_limit", "upload_max_filesize", "post_max_size",
        "max_execution_time", "max_input_time", "display_errors",
        "error_reporting", "date.timezone", "max_file_uploads",
        "session.save_handler", "session.save_path",
    ];

    let mut config = serde_json::Map::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(';') || trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            if keys.contains(&key) {
                config.insert(key.to_string(), json!(value.trim()));
            }
        }
    }

    config.insert("php_ini_path".to_string(), json!(ini_path.display().to_string()));
    Ok(serde_json::to_string_pretty(&config).unwrap())
}

fn tool_set_php_config(version: &str, key: &str, value: &str) -> Result<String, String> {
    if version.is_empty() || key.is_empty() || value.is_empty() {
        return Err("Version, key, and value are required".to_string());
    }

    let bin_dir = get_bin_dir();
    let ini_path = get_php_ini_path(&bin_dir, version)?;
    backup_file(&ini_path)?;

    let content = fs::read_to_string(&ini_path)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    let target_prefix = format!("{} =", key);
    let target_prefix_nospace = format!("{}=", key);
    let new_line = format!("{} = {}", key, value);

    let mut found = false;
    let new_content: String = content.lines().map(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with(';') &&
           (trimmed.starts_with(&target_prefix) || trimmed.starts_with(&target_prefix_nospace)) {
            found = true;
            new_line.clone()
        } else {
            line.to_string()
        }
    }).collect::<Vec<_>>().join("\n");

    if !found {
        let new_content = format!("{}\n{}\n", new_content.trim_end(), new_line);
        fs::write(&ini_path, new_content)
            .map_err(|e| format!("Failed to write php.ini: {}", e))?;
        return Ok(format!("Added {} = {} to PHP {} config", key, value, version));
    }

    fs::write(&ini_path, new_content)
        .map_err(|e| format!("Failed to write php.ini: {}", e))?;

    Ok(format!("Set {} = {} for PHP {}", key, value, version))
}

// ─── Composer Tools ──────────────────────────────────────────────

fn tool_composer_require(project_path: &str, package: &str, dev: bool) -> Result<String, String> {
    if project_path.is_empty() || package.is_empty() {
        return Err("Project path and package are required".to_string());
    }

    let bin_dir = get_bin_dir();
    let php = find_php_exe(&bin_dir)?;
    let composer = find_composer_phar(&bin_dir)?;

    let mut cmd = hidden_command(&php);
    cmd.arg(&composer).arg("require");
    if dev {
        cmd.arg("--dev");
    }
    cmd.arg(package).arg("--no-interaction");
    cmd.current_dir(project_path);

    let output = cmd.output()
        .map_err(|e| format!("Failed to run composer: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(format!("Composer error:\n{}{}", stdout, stderr))
    }
}

fn tool_composer_install(project_path: &str) -> Result<String, String> {
    if project_path.is_empty() {
        return Err("Project path is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let php = find_php_exe(&bin_dir)?;
    let composer = find_composer_phar(&bin_dir)?;

    let output = hidden_command(&php)
        .arg(&composer).arg("install").arg("--no-interaction")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Failed to run composer: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(format!("Composer error:\n{}{}", stdout, stderr))
    }
}

fn tool_composer_run(project_path: &str, script: &str) -> Result<String, String> {
    if project_path.is_empty() || script.is_empty() {
        return Err("Project path and script are required".to_string());
    }

    let bin_dir = get_bin_dir();
    let php = find_php_exe(&bin_dir)?;
    let composer = find_composer_phar(&bin_dir)?;

    let output = hidden_command(&php)
        .arg(&composer).arg("run-script").arg(script).arg("--no-interaction")
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Failed to run composer: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(format!("Composer error:\n{}{}", stdout, stderr))
    }
}

// ─── Redis Tools ─────────────────────────────────────────────────

fn tool_redis_command(command: &str) -> Result<String, String> {
    require_service("redis")?;
    if command.is_empty() {
        return Err("Command is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let redis_cli = find_redis_cli(&bin_dir)?;

    // Split command into args
    let parts: Vec<&str> = command.split_whitespace().collect();

    let output = hidden_command(&redis_cli)
        .arg("-h").arg("127.0.0.1")
        .arg("-p").arg("6379")
        .args(&parts)
        .output()
        .map_err(|e| format!("Failed to run redis-cli: {}. Is Redis running?", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stderr.trim().is_empty() && !output.status.success() {
        return Err(format!("Redis error: {}", stderr.trim()));
    }

    Ok(stdout.to_string())
}

fn tool_redis_info() -> Result<String, String> {
    require_service("redis")?;
    let bin_dir = get_bin_dir();
    let redis_cli = find_redis_cli(&bin_dir)?;

    let output = hidden_command(&redis_cli)
        .arg("-h").arg("127.0.0.1")
        .arg("-p").arg("6379")
        .arg("INFO")
        .output()
        .map_err(|e| format!("Failed to run redis-cli: {}. Is Redis running?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Redis error: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ─── Mailpit Tools ───────────────────────────────────────────────

fn mailpit_http(method: &str, path: &str) -> Result<String, String> {
    use std::io::{Read as IoRead, Write as StreamWrite};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect("127.0.0.1:8025")
        .map_err(|e| format!("Failed to connect to Mailpit: {}. Is Mailpit running?", e))?;

    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();

    let request = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:8025\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        method, path
    );

    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Failed to send request: {}", e))?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let response_str = String::from_utf8_lossy(&response);

    // Extract body after \r\n\r\n
    if let Some(body_start) = response_str.find("\r\n\r\n") {
        let body = &response_str[body_start + 4..];
        // Handle chunked transfer encoding
        if response_str.contains("Transfer-Encoding: chunked") {
            // Simple chunked decoder: skip chunk sizes
            let mut decoded = String::new();
            let mut remaining = body;
            loop {
                let remaining_trimmed = remaining.trim_start();
                if let Some(newline_pos) = remaining_trimmed.find("\r\n") {
                    let size_str = &remaining_trimmed[..newline_pos];
                    let chunk_size = usize::from_str_radix(size_str.trim(), 16).unwrap_or(0);
                    if chunk_size == 0 {
                        break;
                    }
                    let data_start = newline_pos + 2;
                    let data_end = data_start + chunk_size;
                    if data_end <= remaining_trimmed.len() {
                        decoded.push_str(&remaining_trimmed[data_start..data_end]);
                        remaining = &remaining_trimmed[data_end..];
                    } else {
                        decoded.push_str(&remaining_trimmed[data_start..]);
                        break;
                    }
                } else {
                    decoded.push_str(remaining_trimmed);
                    break;
                }
            }
            Ok(decoded)
        } else {
            Ok(body.to_string())
        }
    } else {
        Ok(response_str.to_string())
    }
}

fn tool_list_emails(limit: usize) -> Result<String, String> {
    require_service("mailpit")?;
    let response = mailpit_http("GET", &format!("/api/v1/messages?limit={}", limit))?;

    // Try to pretty-print JSON
    if let Ok(parsed) = serde_json::from_str::<Value>(&response) {
        Ok(serde_json::to_string_pretty(&parsed).unwrap())
    } else {
        Ok(response)
    }
}

fn tool_get_email(id: &str) -> Result<String, String> {
    require_service("mailpit")?;
    if id.is_empty() {
        return Err("Email ID is required".to_string());
    }

    let response = mailpit_http("GET", &format!("/api/v1/message/{}", id))?;

    if let Ok(parsed) = serde_json::from_str::<Value>(&response) {
        Ok(serde_json::to_string_pretty(&parsed).unwrap())
    } else {
        Ok(response)
    }
}

fn tool_delete_emails() -> Result<String, String> {
    require_service("mailpit")?;
    mailpit_http("DELETE", "/api/v1/messages")?;
    Ok("All emails deleted from Mailpit".to_string())
}

// ─── Config File Tools ───────────────────────────────────────────

fn get_config_file_path(config_type: &str, php_version: Option<&str>) -> Result<PathBuf, String> {
    let bin_dir = get_bin_dir();
    match config_type {
        "nginx" => Ok(bin_dir.join("nginx").join("conf").join("nginx.conf")),
        "apache" => Ok(bin_dir.join("apache").join("conf").join("httpd.conf")),
        "mariadb" => Ok(bin_dir.join("data").join("mariadb").join("my.ini")),
        "php" => {
            let version = php_version.ok_or("php_version is required when type is php")?;
            Ok(bin_dir.join("php").join(version).join("php.ini"))
        }
        _ => Err(format!("Unknown config type: {}. Use: nginx, apache, mariadb, php", config_type)),
    }
}

fn tool_read_config(config_type: &str, php_version: Option<&str>) -> Result<String, String> {
    if config_type.is_empty() {
        return Err("Config type is required".to_string());
    }

    let path = get_config_file_path(config_type, php_version)?;
    if !path.exists() {
        return Err(format!("Config file not found: {}", path.display()));
    }

    fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config: {}", e))
}

fn tool_write_config(config_type: &str, content: &str, php_version: Option<&str>) -> Result<String, String> {
    if config_type.is_empty() || content.is_empty() {
        return Err("Config type and content are required".to_string());
    }

    let path = get_config_file_path(config_type, php_version)?;
    backup_file(&path)?;

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(format!("Config written to {} (backup saved as .bak)", path.display()))
}

fn tool_read_site_config(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }

    let bin_dir = get_bin_dir();

    // Try nginx first
    let nginx_conf = bin_dir.join("nginx").join("conf").join("sites-enabled").join(format!("{}.conf", domain));
    if nginx_conf.exists() {
        return fs::read_to_string(&nginx_conf)
            .map_err(|e| format!("Failed to read config: {}", e));
    }

    // Try apache
    let apache_conf = bin_dir.join("apache").join("conf").join("vhosts").join(format!("{}.conf", domain));
    if apache_conf.exists() {
        return fs::read_to_string(&apache_conf)
            .map_err(|e| format!("Failed to read config: {}", e));
    }

    Err(format!("No site config found for '{}'. Looked in nginx/conf/sites-enabled/ and apache/conf/vhosts/", domain))
}

fn tool_write_site_config(domain: &str, content: &str) -> Result<String, String> {
    if domain.is_empty() || content.is_empty() {
        return Err("Domain and content are required".to_string());
    }

    let bin_dir = get_bin_dir();
    let conf_path = bin_dir.join("nginx").join("conf").join("sites-enabled").join(format!("{}.conf", domain));

    backup_file(&conf_path)?;

    fs::write(&conf_path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Test nginx config
    if is_service_running("nginx") {
        let nginx = find_nginx_exe(&bin_dir)?;
        let test = hidden_command(&nginx)
            .arg("-t")
            .output()
            .map_err(|e| format!("Failed to test nginx config: {}", e))?;

        if !test.status.success() {
            // Rollback
            let bak_path = conf_path.with_extension("conf.bak");
            if bak_path.exists() {
                fs::copy(&bak_path, &conf_path).ok();
            }
            let stderr = String::from_utf8_lossy(&test.stderr);
            return Err(format!("Nginx config test failed (rolled back): {}", stderr.trim()));
        }

        // Reload
        hidden_command(&nginx)
            .args(["-s", "reload"])
            .output()
            .ok();
    }

    Ok(format!("Site config for '{}' updated and nginx reloaded", domain))
}

// ─── Batch Operations ────────────────────────────────────────────

fn tool_start_all_services() -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let services = scan_services(&bin_dir);
    let startable = ["nginx", "php", "mariadb", "redis", "apache", "mailpit", "postgresql", "mongodb"];

    let targets: Vec<&ServiceInfo> = services.iter()
        .filter(|s| startable.contains(&s.service_type.as_str()))
        .collect();

    if targets.is_empty() {
        return Ok("No startable services installed.".to_string());
    }

    let mut results = Vec::new();
    for svc in &targets {
        if is_service_running(&svc.name) {
            results.push(format!("{}: already running", svc.name));
            continue;
        }
        match start_service_process(svc) {
            Ok(pid) => {
                std::thread::sleep(std::time::Duration::from_millis(300));
                results.push(format!("{}: started (PID {})", svc.name, pid));
            }
            Err(e) => results.push(format!("{}: failed — {}", svc.name, e)),
        }
    }

    Ok(results.join("\n"))
}

fn tool_stop_all_services() -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let services = scan_services(&bin_dir);
    let startable = ["nginx", "php", "mariadb", "redis", "apache", "mailpit", "postgresql", "mongodb"];

    let targets: Vec<&ServiceInfo> = services.iter()
        .filter(|s| startable.contains(&s.service_type.as_str()) && is_service_running(&s.name))
        .collect();

    if targets.is_empty() {
        return Ok("No running services to stop.".to_string());
    }

    let mut results = Vec::new();
    for svc in &targets {
        match stop_service_process(&svc.name) {
            Ok(_) => results.push(format!("{}: stopped", svc.name)),
            Err(e) => results.push(format!("{}: failed — {}", svc.name, e)),
        }
    }

    Ok(results.join("\n"))
}

// ─── Hosts File ──────────────────────────────────────────────────

fn get_hosts_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    { PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts") }
    #[cfg(not(target_os = "windows"))]
    { PathBuf::from("/etc/hosts") }
}

fn tool_hosts_list() -> Result<String, String> {
    let hosts_path = get_hosts_path();
    let content = fs::read_to_string(&hosts_path)
        .map_err(|e| format!("Failed to read hosts file: {}", e))?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            entries.push(json!({
                "ip": parts[0],
                "domain": parts[1],
                "local": parts[0] == "127.0.0.1" || parts[0] == "::1"
            }));
        }
    }

    Ok(serde_json::to_string_pretty(&entries).unwrap())
}

fn tool_hosts_add(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }
    add_hosts_entry(domain)?;
    Ok(format!("Added '{}' → 127.0.0.1 to hosts file", domain))
}

fn tool_hosts_remove(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }
    remove_hosts_entry(domain)?;
    Ok(format!("Removed '{}' from hosts file", domain))
}

// ─── Database Export/Import ──────────────────────────────────────

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

fn tool_db_export(database: &str, output: Option<&str>) -> Result<String, String> {
    require_service("mariadb")?;
    if database.is_empty() {
        return Err("Database name is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let dump_exe = find_mariadb_dump(&bin_dir)?;
    let out_file = output.map(|s| s.to_string()).unwrap_or_else(|| format!("{}.sql", database));

    let result = hidden_command(&dump_exe)
        .arg("--host=127.0.0.1").arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg("--routines").arg("--triggers").arg("--single-transaction")
        .arg(database)
        .output()
        .map_err(|e| format!("Failed to run mysqldump: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("Export failed: {}", stderr.trim()));
    }

    fs::write(&out_file, &result.stdout)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    let size = result.stdout.len();
    Ok(format!("Exported '{}' to {} ({} bytes)", database, out_file, size))
}

fn tool_db_import(database: &str, file: &str) -> Result<String, String> {
    require_service("mariadb")?;
    if database.is_empty() || file.is_empty() {
        return Err("Database name and file path are required".to_string());
    }

    let file_path = std::path::Path::new(file);
    if !file_path.exists() {
        return Err(format!("SQL file not found: {}", file));
    }

    let bin_dir = get_bin_dir();
    let client = find_mariadb_client(&bin_dir)?;
    let sql_content = fs::read(file)
        .map_err(|e| format!("Failed to read SQL file: {}", e))?;
    let file_size = sql_content.len();

    let mut child = hidden_command(&client)
        .arg("--host=127.0.0.1").arg("--port=3306")
        .arg("-u").arg("root").arg("-proot")
        .arg(database)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start mysql client: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(&sql_content)
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to wait for import: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Import failed: {}", stderr.trim()));
    }

    Ok(format!("Imported {} ({} bytes) into '{}'", file, file_size, database))
}

// ─── Log Management ─────────────────────────────────────────────

fn tool_clear_log(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Log name is required".to_string());
    }

    // Prevent path traversal
    if name.contains("..") {
        return Err("Invalid log name".to_string());
    }

    let log_dir = get_orbit_data_dir().join("logs");
    let log_path = log_dir.join(name);

    if !log_path.exists() {
        return Err(format!("Log file not found: {}", name));
    }

    // Verify it's inside the logs directory
    if !log_path.starts_with(&log_dir) {
        return Err("Invalid log path".to_string());
    }

    fs::write(&log_path, "")
        .map_err(|e| format!("Failed to clear log: {}", e))?;

    Ok(format!("Cleared log file: {}", name))
}

// ─── Service Install/Uninstall ──────────────────────────────────

fn tool_install_service(service: &str, version: Option<&str>) -> Result<String, String> {
    if service.is_empty() {
        return Err("Service name is required".to_string());
    }

    // Use orbit-cli to handle the install (it has registry, download, extraction logic)
    let cli_exe = find_orbit_cli();

    let mut cmd_args = vec!["install".to_string(), service.to_string()];
    if let Some(ver) = version {
        cmd_args.push("--version".to_string());
        cmd_args.push(ver.to_string());
    }

    let result = hidden_command(&cli_exe)
        .args(&cmd_args)
        .output()
        .map_err(|e| format!("Failed to run orbit-cli install: {}", e))?;

    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    if !result.status.success() && stdout.is_empty() {
        return Err(format!("Install failed: {}", stderr.trim()));
    }

    // Strip ANSI codes from output
    let clean_output = strip_ansi_codes(&stdout);
    Ok(clean_output.trim().to_string())
}

fn tool_uninstall_service(service: &str) -> Result<String, String> {
    if service.is_empty() {
        return Err("Service name is required".to_string());
    }

    let resolved = resolve_service_name(service);
    let bin_dir = get_bin_dir();

    // Stop the service first if running
    if is_service_running(&resolved) {
        stop_service_process(&resolved).ok();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // Determine the service directory
    let service_dir = if resolved.starts_with("php-") {
        let ver = resolved.strip_prefix("php-").unwrap_or("8.4");
        bin_dir.join("php").join(ver)
    } else {
        bin_dir.join(&resolved)
    };

    if !service_dir.exists() {
        return Err(format!("Service '{}' is not installed", service));
    }

    fs::remove_dir_all(&service_dir)
        .map_err(|e| format!("Failed to remove {}: {}", resolved, e))?;

    Ok(format!("Uninstalled '{}' (removed {})", resolved, service_dir.display()))
}

fn find_orbit_cli() -> PathBuf {
    // Check common locations for orbit-cli
    let bin_dir = get_bin_dir();

    // Same directory as MCP binary
    let mcp_dir = bin_dir.join("mcp");
    let cli_in_mcp = mcp_dir.join("orbit-cli.exe");
    if cli_in_mcp.exists() {
        return cli_in_mcp;
    }

    // Parent bin directory
    let cli_in_bin = bin_dir.join("orbit-cli.exe");
    if cli_in_bin.exists() {
        return cli_in_bin;
    }

    // Try PATH
    PathBuf::from("orbit-cli")
}

fn strip_ansi_codes(s: &str) -> String {
    // Simple ANSI escape code stripper
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

// ─── AI Diagnostics Tools ────────────────────────────────────────

fn tool_diagnose_service(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Service name is required".to_string());
    }

    let resolved = resolve_service_name(name);
    let bin_dir = get_bin_dir();
    let services = scan_services(&bin_dir);

    let service = services.iter().find(|s| {
        s.name == resolved || s.name.starts_with(&resolved)
    });

    let svc = match service {
        Some(s) => s,
        None => {
            return Ok(serde_json::to_string_pretty(&json!({
                "service": resolved,
                "status": "not_installed",
                "issues": ["Service is not installed"],
                "suggestions": ["Install the service using: install_service"],
                "details": {}
            })).unwrap());
        }
    };

    let mut issues: Vec<String> = Vec::new();
    let mut suggestions: Vec<String> = Vec::new();
    let mut details = serde_json::Map::new();

    // Check binary
    let exe_exists = std::path::Path::new(&svc.path).exists();
    details.insert("binary_exists".into(), json!(exe_exists));
    if !exe_exists {
        issues.push("Binary not found".into());
        suggestions.push("Reinstall the service".into());
    }

    // Check port
    let port = get_service_port(&svc.name);
    let running = is_service_running(&svc.name);
    details.insert("port".into(), json!(port));
    details.insert("running".into(), json!(running));

    if !running {
        issues.push(format!("{} is not running", svc.name));
        suggestions.push(format!("Start it: start_service {{ \"name\": \"{}\" }}", svc.name));
    }

    // Service-specific checks
    match svc.service_type.as_str() {
        "nginx" => {
            // nginx -t
            let nginx_exe = PathBuf::from(&svc.path);
            if let Ok(output) = hidden_command(&nginx_exe).arg("-t").output() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let config_ok = output.status.success();
                details.insert("config_test".into(), json!(if config_ok { "ok" } else { "failed" }));
                if !config_ok {
                    issues.push(format!("Nginx config test failed: {}", stderr.trim()));
                    suggestions.push("Check nginx config: read_config {{ \"type\": \"nginx\" }}".into());
                }
            }
            // Check error log
            let err_log = bin_dir.join("nginx").join("logs").join("error.log");
            if err_log.exists() {
                let size = fs::metadata(&err_log).map(|m| m.len()).unwrap_or(0);
                details.insert("error_log_size".into(), json!(format_size(size)));
                if size > 100 * 1024 * 1024 {
                    issues.push("Error log is very large (>100MB)".into());
                    suggestions.push("Clear the error log: clear_log {{ \"name\": \"nginx/error.log\" }}".into());
                }
                // Read last few lines for recent errors
                if let Ok(content) = fs::read_to_string(&err_log) {
                    let lines: Vec<&str> = content.lines().collect();
                    let recent: Vec<&str> = lines.iter().rev().take(5).copied().collect();
                    if !recent.is_empty() {
                        details.insert("recent_errors".into(), json!(recent));
                    }
                }
            }
        }
        "php" => {
            let version = svc.name.strip_prefix("php-").unwrap_or("8.4");
            let ini_path = bin_dir.join("php").join(version).join("php.ini");
            details.insert("php_ini_exists".into(), json!(ini_path.exists()));
            if !ini_path.exists() {
                issues.push("php.ini not found".into());
                suggestions.push("Create php.ini from php.ini-development".into());
            } else if let Ok(content) = fs::read_to_string(&ini_path) {
                // Check critical settings
                for key in &["memory_limit", "upload_max_filesize", "max_execution_time"] {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if !trimmed.starts_with(';') && trimmed.starts_with(key) {
                            if let Some((_, val)) = trimmed.split_once('=') {
                                details.insert(key.to_string(), json!(val.trim()));
                            }
                        }
                    }
                }
            }
            // Check PHP error log
            let err_log = bin_dir.join("php").join(version).join("logs").join("php_errors.log");
            if err_log.exists() {
                let size = fs::metadata(&err_log).map(|m| m.len()).unwrap_or(0);
                details.insert("error_log_size".into(), json!(format_size(size)));
            }
        }
        "mariadb" => {
            let data_dir = bin_dir.join("data").join("mariadb");
            details.insert("data_dir_exists".into(), json!(data_dir.exists()));
            if !data_dir.exists() {
                issues.push("MariaDB data directory not found".into());
                suggestions.push("Initialize MariaDB data directory".into());
            }
            // Check error log
            let err_log = data_dir.join("mysql.err");
            if err_log.exists() {
                let size = fs::metadata(&err_log).map(|m| m.len()).unwrap_or(0);
                details.insert("error_log_size".into(), json!(format_size(size)));
            }
            // Try mysqladmin ping if running
            if running {
                if let Ok(client) = find_mariadb_client(&bin_dir) {
                    let ping = hidden_command(&client)
                        .args(["--host=127.0.0.1", "--port=3306", "-u", "root", "-proot", "--connect-timeout=3"])
                        .arg("-e").arg("SELECT 1")
                        .output();
                    if let Ok(output) = ping {
                        let reachable = output.status.success();
                        details.insert("reachable".into(), json!(reachable));
                        if !reachable {
                            issues.push("MariaDB is running but not responding to queries".into());
                            suggestions.push("Check MariaDB error log".into());
                        }
                    }
                }
            }
        }
        "redis" => {
            if running {
                if let Ok(cli) = find_redis_cli(&bin_dir) {
                    let ping = hidden_command(&cli)
                        .args(["-h", "127.0.0.1", "-p", "6379", "PING"])
                        .output();
                    if let Ok(output) = ping {
                        let pong = String::from_utf8_lossy(&output.stdout).contains("PONG");
                        details.insert("ping".into(), json!(if pong { "PONG" } else { "failed" }));
                        if !pong {
                            issues.push("Redis is running but not responding to PING".into());
                        }
                    }
                }
            }
            let redis_log = bin_dir.join("redis").join("redis.log");
            if redis_log.exists() {
                let size = fs::metadata(&redis_log).map(|m| m.len()).unwrap_or(0);
                details.insert("log_size".into(), json!(format_size(size)));
            }
        }
        "apache" => {
            let apache_exe = PathBuf::from(&svc.path);
            if let Ok(output) = hidden_command(&apache_exe).arg("-t").output() {
                let config_ok = output.status.success();
                details.insert("config_test".into(), json!(if config_ok { "ok" } else { "failed" }));
                if !config_ok {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    issues.push(format!("Apache config test failed: {}", stderr.trim()));
                }
            }
        }
        "postgresql" => {
            let data_dir = bin_dir.join("data").join("postgres");
            details.insert("data_dir_exists".into(), json!(data_dir.exists()));
            if running {
                if let Ok(psql) = find_psql_client(&bin_dir) {
                    let ping = hidden_command(&psql)
                        .args(["-U", "postgres", "-h", "127.0.0.1", "-p", "5432", "-c", "SELECT 1"])
                        .env("PGPASSWORD", "postgres")
                        .output();
                    if let Ok(output) = ping {
                        let reachable = output.status.success();
                        details.insert("reachable".into(), json!(reachable));
                        if !reachable {
                            issues.push("PostgreSQL running but not responding".into());
                        }
                    }
                }
            }
        }
        _ => {}
    }

    let status = if !exe_exists {
        "down"
    } else if issues.is_empty() && running {
        "healthy"
    } else if running {
        "degraded"
    } else {
        "down"
    };

    let result = json!({
        "service": svc.name,
        "version": svc.version,
        "status": status,
        "issues": issues,
        "suggestions": suggestions,
        "details": details
    });

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_diagnose_site(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }

    let bin_dir = get_bin_dir();
    let mut issues: Vec<String> = Vec::new();
    let mut suggestions: Vec<String> = Vec::new();
    let mut details = serde_json::Map::new();

    // Check site config exists
    let store = read_sites_store().unwrap_or(SiteStore { version: "1".into(), sites: vec![] });
    let site = store.sites.iter().find(|s| s.domain == domain);

    let has_config = site.is_some();
    details.insert("in_sites_json".into(), json!(has_config));

    if !has_config {
        issues.push("Site not found in sites.json".into());
        suggestions.push("Create the site: create_site".into());
    }

    // Check nginx config file
    let nginx_conf = bin_dir.join("nginx").join("conf").join("sites-enabled").join(format!("{}.conf", domain));
    let has_nginx_conf = nginx_conf.exists();
    details.insert("nginx_config_exists".into(), json!(has_nginx_conf));
    if !has_nginx_conf {
        issues.push("Nginx config file not found".into());
    }

    // Check web server running
    let nginx_running = is_service_running("nginx");
    details.insert("web_server_running".into(), json!(nginx_running));
    if !nginx_running {
        issues.push("Web server (nginx) is not running".into());
        suggestions.push("Start nginx: start_service {{ \"name\": \"nginx\" }}".into());
    }

    // Check PHP if site uses PHP
    if let Some(s) = site {
        if let Some(ref php_ver) = s.php_version {
            let php_name = format!("php-{}", php_ver);
            let php_running = is_service_running(&php_name);
            details.insert("php_version".into(), json!(php_ver));
            details.insert("php_running".into(), json!(php_running));
            if !php_running {
                issues.push(format!("PHP {} is not running", php_ver));
                suggestions.push(format!("Start PHP: start_service {{ \"name\": \"{}\" }}", php_name));
            }
        }

        // Check document root
        let doc_root = std::path::Path::new(&s.path);
        details.insert("document_root".into(), json!(s.path));
        details.insert("document_root_exists".into(), json!(doc_root.exists()));
        if !doc_root.exists() {
            issues.push("Document root does not exist".into());
            suggestions.push(format!("Create directory: {}", s.path));
        }
    }

    // Check hosts file
    let hosts_path = get_hosts_path();
    let in_hosts = if let Ok(content) = fs::read_to_string(&hosts_path) {
        content.contains(&format!("127.0.0.1 {}", domain))
    } else {
        false
    };
    details.insert("in_hosts_file".into(), json!(in_hosts));
    if !in_hosts {
        issues.push("Domain not in hosts file".into());
        suggestions.push(format!("Add hosts entry: hosts_add {{ \"domain\": \"{}\" }}", domain));
    }

    // Check SSL
    let ssl_cert = bin_dir.join("nginx").join("ssl").join(format!("{}.pem", domain));
    let ssl_key = bin_dir.join("nginx").join("ssl").join(format!("{}-key.pem", domain));
    let has_ssl = ssl_cert.exists() && ssl_key.exists();
    details.insert("ssl_cert_exists".into(), json!(ssl_cert.exists()));
    details.insert("ssl_key_exists".into(), json!(ssl_key.exists()));

    if let Some(s) = site {
        if s.ssl_enabled && !has_ssl {
            issues.push("SSL is enabled but certificate files are missing".into());
            suggestions.push(format!("Generate SSL: generate_ssl {{ \"domain\": \"{}\" }}", domain));
        }
    }

    // Check reachability via TCP
    let port = site.map(|s| s.port).unwrap_or(80);
    let reachable = std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        std::time::Duration::from_secs(2),
    ).is_ok();
    details.insert("reachable".into(), json!(reachable));
    if !reachable && nginx_running {
        issues.push(format!("Site not reachable on port {}", port));
        suggestions.push("Check nginx config and ensure site is properly configured".into());
    }

    let status = if issues.is_empty() {
        "healthy"
    } else if reachable {
        "degraded"
    } else {
        "down"
    };

    let result = json!({
        "domain": domain,
        "status": status,
        "issues": issues,
        "suggestions": suggestions,
        "details": details
    });

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_analyze_logs(service: Option<&str>, lines: usize, severity: &str) -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let all_logs = scan_log_files(&bin_dir);

    let logs: Vec<&LogFile> = if let Some(svc) = service {
        all_logs.iter().filter(|l| l.name.starts_with(svc)).collect()
    } else {
        all_logs.iter().collect()
    };

    if logs.is_empty() {
        return Ok(serde_json::to_string_pretty(&json!({
            "logs_analyzed": 0,
            "total_errors": 0,
            "patterns": [],
            "message": "No log files found"
        })).unwrap());
    }

    // Known error patterns with solutions
    let known_patterns: Vec<(&str, &str)> = vec![
        ("502 Bad Gateway", "PHP-FPM is not running or port mismatch. Check PHP service and port configuration."),
        ("Connection refused", "Target service is not running. Start the required service."),
        ("Permission denied", "File permission issue. Check file ownership and permissions."),
        ("No input file specified", "Document root is incorrect or index file is missing. Check site configuration."),
        ("Allowed memory size", "PHP memory_limit exceeded. Increase memory_limit in php.ini."),
        ("Maximum execution time", "Script timeout. Increase max_execution_time in php.ini."),
        ("upstream timed out", "Backend service too slow to respond. Increase timeout values."),
        ("Address already in use", "Port is already in use by another process. Check for port conflicts."),
        ("SSL_ERROR", "SSL certificate issue. Regenerate SSL certificate with generate_ssl."),
        ("404 Not Found", "Requested file does not exist. Check document root and file paths."),
        ("access denied", "Database access denied. Check credentials in .env or config."),
        ("Can't connect to", "Database connection failed. Ensure database service is running."),
        ("FATAL ERROR", "Fatal application error. Check application logs for stack trace."),
        ("Segmentation fault", "Process crashed. Check for corrupted binaries or incompatible extensions."),
        ("[error]", "General error entry."),
        ("[warn]", "Warning entry."),
        ("[crit]", "Critical error."),
        ("[emerg]", "Emergency — service may be unusable."),
    ];

    let mut all_patterns: std::collections::HashMap<String, (usize, String)> = std::collections::HashMap::new();
    let mut total_errors = 0;
    let mut analyzed_logs = Vec::new();

    for log in &logs {
        if let Ok(content) = fs::read_to_string(&log.path) {
            let all_lines: Vec<&str> = content.lines().collect();
            let start = if all_lines.len() > lines { all_lines.len() - lines } else { 0 };
            let tail = &all_lines[start..];

            let mut log_errors = 0;
            for line in tail {
                let lower = line.to_lowercase();

                // Filter by severity
                let is_error = lower.contains("error") || lower.contains("fatal") ||
                    lower.contains("crit") || lower.contains("emerg") || lower.contains("fail");
                let is_warning = lower.contains("warn");

                let include = match severity {
                    "all" => true,
                    "warning" => is_error || is_warning,
                    _ => is_error, // "error" default
                };

                if !include { continue; }

                log_errors += 1;
                total_errors += 1;

                // Match known patterns
                for (pattern, solution) in &known_patterns {
                    if lower.contains(&pattern.to_lowercase()) {
                        let entry = all_patterns.entry(pattern.to_string())
                            .or_insert((0, solution.to_string()));
                        entry.0 += 1;
                    }
                }
            }

            analyzed_logs.push(json!({
                "log": log.name,
                "lines_analyzed": tail.len(),
                "errors_found": log_errors,
                "size": format_size(log.size)
            }));
        }
    }

    // Sort patterns by frequency
    let mut pattern_list: Vec<Value> = all_patterns.iter()
        .map(|(pattern, (count, solution))| json!({
            "pattern": pattern,
            "count": count,
            "suggestion": solution
        }))
        .collect();
    pattern_list.sort_by(|a, b| {
        let ca = a.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
        let cb = b.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
        cb.cmp(&ca)
    });

    // Top 20
    pattern_list.truncate(20);

    let result = json!({
        "logs_analyzed": analyzed_logs.len(),
        "total_errors": total_errors,
        "severity_filter": severity,
        "logs": analyzed_logs,
        "patterns": pattern_list
    });

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_get_health_report() -> Result<String, String> {
    let bin_dir = get_bin_dir();
    let data_dir = get_orbit_data_dir();
    let services = scan_services(&bin_dir);
    let logs = scan_log_files(&bin_dir);
    let store = read_sites_store().unwrap_or(SiteStore { version: "1".into(), sites: vec![] });

    let mut score: i32 = 100;
    let mut issues: Vec<String> = Vec::new();
    let mut service_list = Vec::new();

    // Check all services
    for svc in &services {
        let running = is_service_running(&svc.name);
        let port = get_service_port(&svc.name);
        service_list.push(json!({
            "name": svc.name,
            "version": svc.version,
            "status": if running { "running" } else { "stopped" },
            "port": port
        }));
    }

    // Check for port conflicts
    let mut port_map: std::collections::HashMap<u16, Vec<String>> = std::collections::HashMap::new();
    for svc in &services {
        if let Some(port) = get_service_port(&svc.name) {
            port_map.entry(port).or_default().push(svc.name.clone());
        }
    }
    let mut port_conflicts = Vec::new();
    for (port, svcs) in &port_map {
        if svcs.len() > 1 {
            port_conflicts.push(json!({
                "port": port,
                "services": svcs
            }));
            score -= 15;
            issues.push(format!("Port {} conflict: {}", port, svcs.join(", ")));
        }
    }

    // Disk usage for bin/ and data/
    let bin_size = dir_size(&bin_dir);
    let data_size = dir_size(&data_dir.join("data"));

    // Large log files (>100MB)
    let mut large_logs = Vec::new();
    for log in &logs {
        if log.size > 100 * 1024 * 1024 {
            large_logs.push(json!({
                "name": log.name,
                "size": format_size(log.size)
            }));
            score -= 5;
            issues.push(format!("Large log file: {} ({})", log.name, format_size(log.size)));
        }
    }

    // Site health
    let mut site_issues_list = Vec::new();
    for site in &store.sites {
        let mut site_problems: Vec<String> = Vec::new();
        let doc_root = std::path::Path::new(&site.path);
        if !doc_root.exists() {
            site_problems.push("Document root missing".into());
            score -= 5;
        }
        if let Some(ref php_ver) = site.php_version {
            let php_name = format!("php-{}", php_ver);
            if !is_service_running(&php_name) {
                site_problems.push(format!("PHP {} not running", php_ver));
                score -= 3;
            }
        }
        if !site_problems.is_empty() {
            site_issues_list.push(json!({
                "domain": site.domain,
                "issues": site_problems
            }));
        }
    }

    // Check key services not running
    let key_services = ["nginx", "mariadb"];
    for key in key_services {
        let installed = services.iter().any(|s| s.service_type == key);
        if installed && !is_service_running(key) {
            score -= 10;
            issues.push(format!("{} is installed but not running", key));
        }
    }

    // Clamp score
    if score < 0 { score = 0; }

    let result = json!({
        "score": score,
        "status": if score >= 80 { "good" } else if score >= 50 { "fair" } else { "poor" },
        "services": service_list,
        "port_conflicts": port_conflicts,
        "disk_usage": {
            "bin_directory": format_size(bin_size),
            "data_directory": format_size(data_size)
        },
        "large_logs": large_logs,
        "site_issues": site_issues_list,
        "issues": issues,
        "sites_count": store.sites.len(),
        "services_count": services.len(),
        "services_running": services.iter().filter(|s| is_service_running(&s.name)).count()
    });

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn dir_size(path: &std::path::Path) -> u64 {
    if !path.exists() { return 0; }
    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path);
            } else {
                total += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

// ─── Blueprint System Tools ─────────────────────────────────────

#[derive(Clone)]
struct Blueprint {
    name: &'static str,
    description: &'static str,
    services: &'static [&'static str],
    template: &'static str,
    scaffold: &'static [&'static str],
    php_extensions: &'static [&'static str],
    env_template: Option<&'static str>,
    dev_command: Option<&'static str>,
}

fn get_blueprints() -> Vec<Blueprint> {
    vec![
        Blueprint {
            name: "laravel-vite",
            description: "Laravel with Vite frontend bundler, MariaDB, and Redis",
            services: &["nginx", "php", "mariadb", "redis"],
            template: "laravel",
            scaffold: &["composer create-project laravel/laravel .", "npm install"],
            php_extensions: &["pdo_mysql", "mbstring", "openssl", "tokenizer", "xml", "ctype", "json", "bcmath", "redis"],
            env_template: Some("APP_NAME={{domain}}\nAPP_URL=http://{{domain}}\nDB_CONNECTION=mysql\nDB_HOST=127.0.0.1\nDB_PORT=3306\nDB_DATABASE={{db_name}}\nDB_USERNAME=root\nDB_PASSWORD=root\nCACHE_DRIVER=redis\nSESSION_DRIVER=redis\nREDIS_HOST=127.0.0.1\n"),
            dev_command: Some("npm run dev"),
        },
        Blueprint {
            name: "wordpress-woocommerce",
            description: "WordPress with WooCommerce-ready configuration",
            services: &["nginx", "php", "mariadb"],
            template: "wordpress",
            scaffold: &["composer create-project johnpbloch/wordpress ."],
            php_extensions: &["pdo_mysql", "gd", "mbstring", "xml", "curl", "zip", "intl"],
            env_template: None,
            dev_command: None, // WordPress runs via PHP-FPM, no app process needed
        },
        Blueprint {
            name: "nextjs-fullstack",
            description: "Next.js full-stack application with nginx reverse proxy",
            services: &["nginx", "nodejs"],
            template: "reverse-proxy",
            scaffold: &["npx create-next-app@latest . --yes"],
            php_extensions: &[],
            env_template: None,
            dev_command: Some("npm run dev"),
        },
        Blueprint {
            name: "astro-static",
            description: "Astro static site generator",
            services: &["nginx"],
            template: "static",
            scaffold: &["npm create astro@latest . -- --yes"],
            php_extensions: &[],
            env_template: None,
            dev_command: Some("npm run dev"),
        },
        Blueprint {
            name: "django",
            description: "Django web framework with nginx reverse proxy",
            services: &["nginx", "python"],
            template: "django",
            scaffold: &["pip install django", "django-admin startproject app ."],
            php_extensions: &[],
            env_template: Some("DEBUG=True\nSECRET_KEY=change-me\nALLOWED_HOSTS={{domain}},localhost,127.0.0.1\nDATABASE_URL=sqlite:///db.sqlite3\n"),
            dev_command: Some("python manage.py runserver"),
        },
        Blueprint {
            name: "flask",
            description: "Flask micro web framework with nginx reverse proxy",
            services: &["nginx", "python"],
            template: "django",
            scaffold: &["pip install flask"],
            php_extensions: &[],
            env_template: Some("FLASK_APP=app.py\nFLASK_ENV=development\nFLASK_DEBUG=1\n"),
            dev_command: Some("python -m flask run"),
        },
        Blueprint {
            name: "sveltekit",
            description: "SvelteKit application with nginx reverse proxy and WebSocket support",
            services: &["nginx", "nodejs"],
            template: "sveltekit",
            scaffold: &["npm create svelte@latest . -- --yes"],
            php_extensions: &[],
            env_template: None,
            dev_command: Some("npm run dev"),
        },
        Blueprint {
            name: "remix",
            description: "Remix full-stack web framework",
            services: &["nginx", "nodejs"],
            template: "remix",
            scaffold: &["npx create-remix@latest . --yes"],
            php_extensions: &[],
            env_template: None,
            dev_command: Some("npm run dev"),
        },
    ]
}

fn tool_list_blueprints() -> Result<String, String> {
    let blueprints = get_blueprints();
    let result: Vec<Value> = blueprints.iter().map(|bp| json!({
        "name": bp.name,
        "description": bp.description,
        "services": bp.services,
        "template": bp.template
    })).collect();

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

fn tool_get_blueprint(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Blueprint name is required".to_string());
    }

    let blueprints = get_blueprints();
    let bp = blueprints.iter().find(|b| b.name == name);

    match bp {
        Some(bp) => {
            let result = json!({
                "name": bp.name,
                "description": bp.description,
                "services": bp.services,
                "template": bp.template,
                "scaffold_commands": bp.scaffold,
                "php_extensions": bp.php_extensions,
                "has_env_template": bp.env_template.is_some()
            });
            Ok(serde_json::to_string_pretty(&result).unwrap())
        }
        None => {
            let available: Vec<&str> = blueprints.iter().map(|b| b.name).collect();
            Err(format!("Blueprint '{}' not found. Available: {}", name, available.join(", ")))
        }
    }
}

fn tool_create_from_blueprint(
    blueprint_name: &str,
    domain: &str,
    path: &str,
    php_version: Option<&str>,
) -> Result<String, String> {
    if blueprint_name.is_empty() || domain.is_empty() || path.is_empty() {
        return Err("Blueprint, domain, and path are required".to_string());
    }

    let blueprints = get_blueprints();
    let bp = blueprints.iter().find(|b| b.name == blueprint_name)
        .ok_or_else(|| {
            let available: Vec<&str> = blueprints.iter().map(|b| b.name).collect();
            format!("Blueprint '{}' not found. Available: {}", blueprint_name, available.join(", "))
        })?;

    let bin_dir = get_bin_dir();
    let services = scan_services(&bin_dir);
    let php_ver = php_version.unwrap_or("8.4");
    let mut steps: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Step 1: Verify required services are installed
    for required in bp.services {
        let resolved = if *required == "php" {
            format!("php-{}", php_ver)
        } else {
            required.to_string()
        };

        let found = services.iter().any(|s| {
            s.name == resolved || s.service_type == *required
        });

        if !found {
            return Err(format!(
                "Required service '{}' is not installed. Install it first: install_service {{ \"service\": \"{}\" }}",
                required, required
            ));
        }
    }
    steps.push("Verified all required services are installed".into());

    // Step 2: Start services that aren't running
    for required in bp.services {
        let resolved = if *required == "php" {
            format!("php-{}", php_ver)
        } else {
            required.to_string()
        };

        let svc = services.iter().find(|s| {
            s.name == resolved || s.service_type == *required
        });

        if let Some(svc) = svc {
            if !is_service_running(&svc.name) {
                match start_service_process(svc) {
                    Ok(pid) => {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        steps.push(format!("Started {} (PID: {})", svc.name, pid));
                    }
                    Err(e) => warnings.push(format!("Failed to start {}: {}", svc.name, e)),
                }
            } else {
                steps.push(format!("{} already running", svc.name));
            }
        }
    }

    // Step 3: Enable PHP extensions if needed
    if !bp.php_extensions.is_empty() {
        let ini_path = bin_dir.join("php").join(php_ver).join("php.ini");
        if ini_path.exists() {
            if let Ok(content) = fs::read_to_string(&ini_path) {
                let mut new_content = content.clone();
                let mut enabled_exts = Vec::new();
                for ext in bp.php_extensions {
                    let disabled = format!(";extension={}", ext);
                    let enabled = format!("extension={}", ext);
                    if new_content.contains(&disabled) {
                        new_content = new_content.replace(&disabled, &enabled);
                        enabled_exts.push(*ext);
                    } else if !new_content.contains(&enabled) {
                        new_content = format!("{}\n{}\n", new_content.trim_end(), enabled);
                        enabled_exts.push(*ext);
                    }
                }
                if !enabled_exts.is_empty() {
                    backup_file(&ini_path).ok();
                    fs::write(&ini_path, &new_content).ok();
                    steps.push(format!("Enabled PHP extensions: {}", enabled_exts.join(", ")));
                }
            }
        }
    }

    // Step 4: Create project directory
    let project_path = std::path::Path::new(path);
    if !project_path.exists() {
        fs::create_dir_all(project_path)
            .map_err(|e| format!("Failed to create project directory: {}", e))?;
        steps.push(format!("Created directory: {}", path));
    }

    // Step 5: Create site
    let site_result = tool_create_site(domain, path, Some(bp.template), Some(php_ver), false);
    match site_result {
        Ok(msg) => steps.push(format!("Created site: {}", msg)),
        Err(e) => {
            if e.contains("already exists") {
                warnings.push(format!("Site already exists: {}", domain));
            } else {
                warnings.push(format!("Site creation warning: {}", e));
            }
        }
    }

    // Step 6: Run scaffold commands
    for cmd_str in bp.scaffold {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() { continue; }

        let program = parts[0];
        let cmd_args = &parts[1..];

        // Find the program
        let program_path = match program {
            "composer" => {
                // Use PHP + composer.phar
                match (find_php_exe(&bin_dir), find_composer_phar(&bin_dir)) {
                    (Ok(php), Ok(phar)) => {
                        let mut cmd = hidden_command(&php);
                        cmd.arg(&phar);
                        for arg in cmd_args {
                            cmd.arg(arg);
                        }
                        cmd.arg("--no-interaction");
                        cmd.current_dir(path);
                        match cmd.output() {
                            Ok(output) => {
                                if output.status.success() {
                                    steps.push(format!("Ran: {}", cmd_str));
                                } else {
                                    let stderr = String::from_utf8_lossy(&output.stderr);
                                    warnings.push(format!("Command '{}' failed: {}", cmd_str, stderr.trim()));
                                }
                            }
                            Err(e) => warnings.push(format!("Failed to run '{}': {}", cmd_str, e)),
                        }
                        continue;
                    }
                    _ => {
                        warnings.push(format!("Composer not available, skipping: {}", cmd_str));
                        continue;
                    }
                }
            }
            "pip" => {
                let python = bin_dir.join("python").join("python.exe");
                if python.exists() {
                    let mut cmd = hidden_command(&python);
                    cmd.arg("-m").arg("pip");
                    for arg in cmd_args {
                        cmd.arg(arg);
                    }
                    cmd.current_dir(path);
                    match cmd.output() {
                        Ok(output) => {
                            if output.status.success() {
                                steps.push(format!("Ran: {}", cmd_str));
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                warnings.push(format!("Command '{}' failed: {}", cmd_str, stderr.trim()));
                            }
                        }
                        Err(e) => warnings.push(format!("Failed to run '{}': {}", cmd_str, e)),
                    }
                    continue;
                }
                warnings.push(format!("Python not available, skipping: {}", cmd_str));
                continue;
            }
            "django-admin" => {
                let python = bin_dir.join("python").join("python.exe");
                if python.exists() {
                    let mut cmd = hidden_command(&python);
                    cmd.arg("-m").arg("django");
                    for arg in cmd_args {
                        cmd.arg(arg);
                    }
                    cmd.current_dir(path);
                    match cmd.output() {
                        Ok(output) => {
                            if output.status.success() {
                                steps.push(format!("Ran: {}", cmd_str));
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                warnings.push(format!("Command '{}' failed: {}", cmd_str, stderr.trim()));
                            }
                        }
                        Err(e) => warnings.push(format!("Failed to run '{}': {}", cmd_str, e)),
                    }
                    continue;
                }
                warnings.push(format!("Python not available for django-admin, skipping: {}", cmd_str));
                continue;
            }
            _ => {
                // Try npx, npm, node from bin/nodejs
                let node_dir = bin_dir.join("nodejs");
                let program_exe = if program == "npx" || program == "npm" {
                    node_dir.join(format!("{}.cmd", program))
                } else {
                    PathBuf::from(program)
                };

                program_exe
            }
        };

        let mut cmd = hidden_command(&program_path);
        for arg in cmd_args {
            cmd.arg(arg);
        }
        cmd.current_dir(path);

        // Add node_modules/.bin to PATH
        let node_dir = bin_dir.join("nodejs");
        if node_dir.exists() {
            let current_path = std::env::var("PATH").unwrap_or_default();
            cmd.env("PATH", format!("{};{}", node_dir.display(), current_path));
        }

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    steps.push(format!("Ran: {}", cmd_str));
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warnings.push(format!("Command '{}' failed: {}", cmd_str, stderr.trim()));
                }
            }
            Err(e) => warnings.push(format!("Failed to run '{}': {}", cmd_str, e)),
        }
    }

    // Step 7: Write .env if template exists
    if let Some(env_tpl) = bp.env_template {
        let db_name = domain.replace('.', "_").replace('-', "_");
        let env_content = env_tpl
            .replace("{{domain}}", domain)
            .replace("{{db_name}}", &db_name);

        let env_path = project_path.join(".env");
        if !env_path.exists() {
            fs::write(&env_path, &env_content).ok();
            steps.push("Created .env file".into());
        } else {
            warnings.push(".env file already exists, skipped".into());
        }
    }

    // Step 8: Set dev_command on the site metadata
    if let Some(dev_cmd) = bp.dev_command {
        match read_sites_store() {
            Ok(mut store) => {
                if let Some(site) = store.sites.iter_mut().find(|s| s.domain == domain) {
                    site.dev_command = Some(dev_cmd.to_string());
                    site.updated_at = chrono_now();
                    if let Err(e) = write_sites_store(&store) {
                        warnings.push(format!("Failed to save dev_command: {}", e));
                    } else {
                        steps.push(format!("Set dev_command: {}", dev_cmd));
                    }
                }
            }
            Err(e) => warnings.push(format!("Failed to update dev_command: {}", e)),
        }
    }

    let result = json!({
        "blueprint": bp.name,
        "domain": domain,
        "path": path,
        "dev_command": bp.dev_command,
        "steps": steps,
        "warnings": warnings,
        "status": if warnings.is_empty() { "success" } else { "completed_with_warnings" }
    });

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

// ─── Site App Process Tools ──────────────────────────────────────

fn get_site_app_pid_dir() -> std::path::PathBuf {
    get_config_dir().join("site-pids")
}

fn tool_start_site_app(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }

    let store = read_sites_store()?;
    let site = store.sites.iter().find(|s| s.domain == domain)
        .ok_or_else(|| format!("Site '{}' not found", domain))?;

    let dev_command = site.dev_command.as_ref()
        .ok_or_else(|| format!("Site '{}' has no dev_command configured", domain))?;

    let working_dir = &site.path;
    if !std::path::Path::new(working_dir).exists() {
        return Err(format!("Site directory does not exist: {}", working_dir));
    }

    // Check if already running
    let pid_dir = get_site_app_pid_dir();
    let pid_file = pid_dir.join(format!("{}.pid", domain));
    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Check if process is still alive
                let check = hidden_command("tasklist")
                    .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
                    .output();
                if let Ok(output) = check {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.contains(&pid.to_string()) {
                        return Err(format!("Site app for '{}' is already running (PID: {})", domain, pid));
                    }
                }
                // Process not running, clean up stale PID file
                let _ = fs::remove_file(&pid_file);
            }
        }
    }

    // Parse command
    let parts: Vec<&str> = dev_command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("dev_command is empty".to_string());
    }

    // On Windows, use cmd.exe for npm/npx/bun/python etc.
    let cmd_name = parts[0];
    let needs_shell = matches!(
        cmd_name.to_lowercase().as_str(),
        "npm" | "npx" | "yarn" | "pnpm" | "bun" | "bunx"
            | "python" | "python3" | "pip" | "pip3"
            | "composer" | "php" | "deno" | "go" | "node"
    );

    let mut command = if needs_shell {
        let mut cmd = hidden_command("cmd");
        cmd.arg("/C");
        cmd.arg(dev_command);
        cmd
    } else {
        let mut cmd = hidden_command(cmd_name);
        for arg in &parts[1..] {
            cmd.arg(arg);
        }
        cmd
    };

    command.current_dir(working_dir);

    // Set PORT env var if dev_port is set
    if let Some(port) = site.dev_port {
        command.env("PORT", port.to_string());
    }

    match command.spawn() {
        Ok(child) => {
            let pid = child.id();

            // Save PID to file
            fs::create_dir_all(&pid_dir).ok();
            fs::write(&pid_file, pid.to_string()).ok();

            Ok(serde_json::to_string_pretty(&json!({
                "domain": domain,
                "dev_command": dev_command,
                "pid": pid,
                "status": "started"
            })).unwrap())
        }
        Err(e) => Err(format!("Failed to start site app: {}", e)),
    }
}

fn tool_stop_site_app(domain: &str) -> Result<String, String> {
    if domain.is_empty() {
        return Err("Domain is required".to_string());
    }

    let pid_dir = get_site_app_pid_dir();
    let pid_file = pid_dir.join(format!("{}.pid", domain));

    if !pid_file.exists() {
        return Err(format!("No running app process for site '{}'", domain));
    }

    let pid_str = fs::read_to_string(&pid_file)
        .map_err(|e| format!("Failed to read PID file: {}", e))?;
    let pid = pid_str.trim().parse::<u32>()
        .map_err(|_| "Invalid PID in file".to_string())?;

    // Kill the process tree
    let _ = hidden_command("taskkill")
        .args(&["/F", "/PID", &pid.to_string(), "/T"])
        .output();

    // Clean up PID file
    let _ = fs::remove_file(&pid_file);

    Ok(serde_json::to_string_pretty(&json!({
        "domain": domain,
        "pid": pid,
        "status": "stopped"
    })).unwrap())
}

// ─── Utilities ───────────────────────────────────────────────────

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

// ─── Entry Point ─────────────────────────────────────────────────

fn main() {
    eprintln!("[orbit-mcp] Orbit MCP Server v{} starting...", env!("CARGO_PKG_VERSION"));
    eprintln!("[orbit-mcp] Data dir: {}", get_orbit_data_dir().display());

    // Standby mode: process stays alive without reading stdin
    // Used when started from Orbit GUI (not by an AI tool)
    if std::env::args().any(|a| a == "--standby") {
        eprintln!("[orbit-mcp] Running in standby mode");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

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
