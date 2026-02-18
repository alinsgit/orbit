//! Orbit CLI — Command-line interface for managing local development services
//! 
//! Usage:
//!   orbit-cli status              Show status of all services
//!   orbit-cli start <service>     Start a service
//!   orbit-cli start --all         Start all installed services
//!   orbit-cli stop <service>      Stop a service
//!   orbit-cli stop --all          Stop all services
//!   orbit-cli list                List available services to install

use clap::{Parser, Subcommand};
use colored::*;
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

    services
}

// ─── Process Management ───────────────────────────────────────────

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
        // Extract PHP version for port calculation
        let version_str = name.strip_prefix("php-").unwrap_or("8.4");
        let cleaned: String = version_str.chars().filter(|c| c.is_ascii_digit()).collect();
        let version_num: u32 = cleaned.parse().unwrap_or(84);
        Some(9000 + version_num as u16)
    } else if name.contains("mailpit") {
        Some(8025)
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

    // Determine the actual executable to run
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
        "apache" => {
            (exe_path.clone(), vec![])
        }
        "mailpit" => {
            (exe_path.clone(), vec![])
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
        // Check if port is free (might have already stopped)
        if let Some(port) = get_service_port(name) {
            if !is_port_in_use(port) {
                return Ok(());
            }
        }
        Err(format!("Could not stop {}", name))
    }
}

// ─── CLI Definition ───────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "orbit",
    about = "Orbit — Modern Local Development Environment",
    version = "0.1.5",
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

    /// List all installed services
    List,
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

    // Find max widths for alignment
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
            matches!(s.service_type.as_str(), "nginx" | "php" | "mariadb" | "redis" | "apache" | "mailpit")
        }).collect()
    } else if let Some(ref name) = service_name {
        services.iter().filter(|s| {
            s.name == *name || s.service_type == *name || s.name.starts_with(name)
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

fn cmd_list(bin_dir: &PathBuf) {
    print_header();

    let services = scan_services(bin_dir);

    // All known service types
    let known_services = vec![
        ("nginx", "Nginx", "High-performance web server"),
        ("apache", "Apache", "Classic HTTP server"),
        ("php", "PHP", "Server-side scripting language"),
        ("mariadb", "MariaDB", "MySQL-compatible database"),
        ("redis", "Redis", "In-memory data store"),
        ("nodejs", "Node.js", "JavaScript runtime"),
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

// ─── Main ─────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let bin_dir = get_bin_dir();

    match cli.command {
        Commands::Status => cmd_status(&bin_dir),
        Commands::Start { service, all } => cmd_start(&bin_dir, service, all),
        Commands::Stop { service, all } => cmd_stop(&bin_dir, service, all),
        Commands::List => cmd_list(&bin_dir),
    }
}
