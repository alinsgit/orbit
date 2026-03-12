use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use super::hidden_command;
use crate::services::site_store::SiteStore;

/// AI tool status information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AiToolStatus {
  pub installed: bool,
  pub path: Option<String>,
  pub version: Option<String>,
  /// "orbit" if installed in Orbit's nodejs folder, "system" if found in system PATH
  pub source: Option<String>,
}

pub struct ClaudeCodeManager;

impl ClaudeCodeManager {
  /// Get npm executable path (Orbit's own Node.js)
  pub fn get_npm_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
      .path()
      .app_local_data_dir()
      .map_err(|e| e.to_string())?
      .join("bin")
      .join("nodejs");

    #[cfg(target_os = "windows")]
    return Ok(base.join("npm.cmd"));

    #[cfg(not(target_os = "windows"))]
    return Ok(base.join("bin").join("npm"));
  }

  /// Get claude executable path in Orbit's nodejs folder
  pub fn get_orbit_exe_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
      .path()
      .app_local_data_dir()
      .map_err(|e| e.to_string())?
      .join("bin")
      .join("nodejs");

    #[cfg(target_os = "windows")]
    return Ok(base.join("claude.cmd"));

    #[cfg(not(target_os = "windows"))]
    return Ok(base.join("bin").join("claude"));
  }

  /// Find claude in system PATH (outside Orbit)
  fn find_system_exe() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let cmd = "which";

    let output = hidden_command(cmd)
      .args(["claude"])
      .output()
      .ok()?;

    if output.status.success() {
      let stdout = String::from_utf8_lossy(&output.stdout);
      let first_line = stdout.lines().next()?.trim().to_string();
      if !first_line.is_empty() {
        return Some(PathBuf::from(first_line));
      }
    }
    None
  }

  /// Get version from a specific exe path
  fn get_version_from(exe: &Path) -> Option<String> {
    let output = hidden_command(exe)
      .args(["--version"])
      .output()
      .ok()?;

    if output.status.success() {
      let stdout = String::from_utf8_lossy(&output.stdout);
      let version = stdout.trim().to_string();
      if !version.is_empty() {
        return Some(version);
      }
    }
    None
  }

  /// Get full Claude Code status — checks Orbit first, then system PATH
  pub fn get_status(app: &AppHandle) -> Result<AiToolStatus, String> {
    // 1. Check Orbit's own installation
    let orbit_exe = Self::get_orbit_exe_path(app)?;
    if orbit_exe.exists() {
      let version = Self::get_version_from(&orbit_exe);
      return Ok(AiToolStatus {
        installed: true,
        path: Some(orbit_exe.to_string_lossy().to_string()),
        version,
        source: Some("orbit".to_string()),
      });
    }

    // 2. Check system PATH
    if let Some(system_exe) = Self::find_system_exe() {
      let version = Self::get_version_from(&system_exe);
      return Ok(AiToolStatus {
        installed: true,
        path: Some(system_exe.to_string_lossy().to_string()),
        version,
        source: Some("system".to_string()),
      });
    }

    // 3. Not found anywhere
    Ok(AiToolStatus::default())
  }

  /// Install Claude Code via npm
  pub fn install(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed. Please install Node.js first.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["install", "-g", "@anthropic-ai/claude-code"])
      .output()
      .map_err(|e| format!("Failed to run npm install: {e}"))?;

    if output.status.success() {
      // Best effort: auto-configure orbit-mcp in Claude Code's MCP config
      setup_mcp_for_claude(app).ok();
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }

  /// Uninstall Claude Code via npm
  pub fn uninstall(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["uninstall", "-g", "@anthropic-ai/claude-code"])
      .output()
      .map_err(|e| format!("Failed to run npm uninstall: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }

  /// Update Claude Code via npm
  pub fn update(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["update", "-g", "@anthropic-ai/claude-code"])
      .output()
      .map_err(|e| format!("Failed to run npm update: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }
}

/// Check if a port is in use (service is running)
fn is_port_in_use(port: u16) -> bool {
  TcpListener::bind(format!("127.0.0.1:{port}")).is_err()
    || TcpListener::bind(format!("0.0.0.0:{port}")).is_err()
}

/// Parse PHP minor version from a version string like "8.4" → 4
fn php_port_from_version(php_version: &str) -> u16 {
  let minor = php_version
    .split('.')
    .nth(1)
    .and_then(|s| s.parse::<u16>().ok())
    .unwrap_or(0);
  9000 + minor
}

/// Generate AI context markdown for a site project
pub fn generate_ai_context(
  app: &AppHandle,
  domain: &str,
  data_dir: &Path,
) -> Result<String, String> {
  let store = SiteStore::load(app)?;
  let site = store
    .get_site(domain)
    .ok_or_else(|| format!("Site not found: {domain}"))?;

  let mut md = String::new();

  // --- Project section ---
  md.push_str("# Orbit Context\n\n");
  md.push_str("## Project\n\n");
  md.push_str(&format!("- **Domain**: {}\n", site.domain));
  md.push_str(&format!("- **Path**: {}\n", site.path));
  if let Some(template) = &site.template {
    md.push_str(&format!("- **Template**: {template}\n"));
  }
  if let Some(php) = &site.php_version {
    md.push_str(&format!("- **PHP Version**: {php}\n"));
  }
  md.push_str(&format!("- **Web Server**: {}\n", site.web_server));
  md.push_str(&format!(
    "- **SSL**: {}\n",
    if site.ssl_enabled { "enabled" } else { "disabled" }
  ));
  if let Some(dev_port) = site.dev_port {
    md.push_str(&format!("- **Dev Port**: {dev_port}\n"));
  }
  if let Some(dev_cmd) = &site.dev_command {
    md.push_str(&format!("- **Dev Command**: `{dev_cmd}`\n"));
  }
  md.push('\n');

  // --- Active Services section ---
  md.push_str("## Active Services\n\n");

  let daemon_ports: &[(&str, u16)] = &[
    ("nginx", 80),
    ("apache", 8080),
    ("mariadb", 3306),
    ("postgresql", 5432),
    ("mongodb", 27017),
    ("redis", 6379),
    ("mailpit", 8025),
    ("meilisearch", 7700),
  ];

  let mut active_services: Vec<String> = Vec::new();

  for (name, port) in daemon_ports {
    // Skip apache if nginx is the web server (both use port 80 area, avoid false positives)
    if *name == "apache" && site.web_server == "nginx" {
      continue;
    }
    if *name == "nginx" && site.web_server == "apache" {
      continue;
    }
    if is_port_in_use(*port) {
      active_services.push(format!("- **{name}** (port {port})"));
    }
  }

  // Check PHP if site has a PHP version
  if let Some(php_ver) = &site.php_version {
    let php_port = php_port_from_version(php_ver);
    if is_port_in_use(php_port) {
      active_services.push(format!("- **php-{php_ver}** (port {php_port})"));
    }
  }

  if active_services.is_empty() {
    md.push_str("No services detected as running.\n");
  } else {
    for svc in &active_services {
      md.push_str(svc);
      md.push('\n');
    }
  }
  md.push('\n');

  // --- Deploy Targets section ---
  md.push_str("## Deploy Targets\n\n");
  let deploy_path = data_dir
    .join("config")
    .join("deploy-connections.json");

  if deploy_path.exists() {
    match std::fs::read_to_string(&deploy_path) {
      Ok(content) => {
        match serde_json::from_str::<serde_json::Value>(&content) {
          Ok(json) => {
            // deploy-connections.json is expected to be an array or object keyed by domain
            let targets = if let Some(arr) = json.as_array() {
              // Array of connection objects — filter by domain
              arr
                .iter()
                .filter(|conn| {
                  conn.get("domain")
                    .and_then(|d| d.as_str())
                    .map(|d| d == domain)
                    .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>()
            } else if let Some(obj) = json.as_object() {
              // Object keyed by domain
              obj.get(domain)
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
            } else {
              vec![]
            };

            if targets.is_empty() {
              md.push_str("No deploy targets configured for this project.\n");
            } else {
              for target in &targets {
                let name = target
                  .get("name")
                  .and_then(|v| v.as_str())
                  .unwrap_or("unnamed");
                let protocol = target
                  .get("protocol")
                  .and_then(|v| v.as_str())
                  .unwrap_or("unknown");
                let host = target
                  .get("host")
                  .and_then(|v| v.as_str())
                  .unwrap_or("");
                md.push_str(&format!("- **{name}** ({protocol}) → {host}\n"));
              }
            }
          }
          Err(_) => {
            md.push_str("No deploy targets configured for this project.\n");
          }
        }
      }
      Err(_) => {
        md.push_str("No deploy targets configured for this project.\n");
      }
    }
  } else {
    md.push_str("No deploy targets configured for this project.\n");
  }
  md.push('\n');

  // --- Available MCP Tools section ---
  md.push_str("## Available MCP Tools\n\n");
  md.push_str("Orbit MCP server (`orbit-mcp`) exposes tools across these domains:\n\n");
  md.push_str("- **Services**: list, start, stop, restart, get status, install, uninstall\n");
  md.push_str("- **Sites**: list, get config, create, update, delete, read/write site config\n");
  md.push_str("- **MariaDB**: list databases/tables, execute queries, export/import\n");
  md.push_str("- **PostgreSQL**: list databases/tables, execute queries, create/drop databases\n");
  md.push_str("- **MongoDB**: list databases/collections, execute commands\n");
  md.push_str("- **Redis**: run commands, get info\n");
  md.push_str("- **PHP**: get/set config, toggle extensions, list extensions\n");
  md.push_str("- **SSL**: list certs, generate SSL certificate\n");
  md.push_str("- **Logs**: list log files, read logs, clear logs, analyze logs\n");
  md.push_str("- **Composer**: install dependencies, require/run packages\n");
  md.push_str("- **Mailpit**: list/read/delete emails\n");
  md.push_str("- **Config**: read/write Orbit config files, hosts file management\n");
  md.push('\n');

  // --- Git section (only if .git exists) ---
  let git_dir = std::path::Path::new(&site.path).join(".git");
  if git_dir.exists() {
    md.push_str("## Git\n\n");

    // Current branch
    let branch = hidden_command("git")
      .args(["-C", &site.path, "rev-parse", "--abbrev-ref", "HEAD"])
      .output()
      .ok()
      .and_then(|o| {
        if o.status.success() {
          Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else {
          None
        }
      })
      .unwrap_or_else(|| "unknown".to_string());

    md.push_str(&format!("- **Branch**: {branch}\n"));

    // Remote URL
    let remote = hidden_command("git")
      .args(["-C", &site.path, "remote", "get-url", "origin"])
      .output()
      .ok()
      .and_then(|o| {
        if o.status.success() {
          let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
          if url.is_empty() { None } else { Some(url) }
        } else {
          None
        }
      });

    if let Some(remote_url) = remote {
      md.push_str(&format!("- **Remote**: {remote_url}\n"));
    }

    // Uncommitted changes count
    let changes = hidden_command("git")
      .args(["-C", &site.path, "status", "--porcelain"])
      .output()
      .ok()
      .map(|o| {
        if o.status.success() {
          String::from_utf8_lossy(&o.stdout)
            .lines()
            .count()
        } else {
          0
        }
      })
      .unwrap_or(0);

    md.push_str(&format!("- **Uncommitted Changes**: {changes}\n"));
    md.push('\n');
  }

  Ok(md)
}

/// Write AI context files to the site project directory
pub fn write_context_file(
  app: &AppHandle,
  domain: &str,
  data_dir: &Path,
) -> Result<String, String> {
  let content = generate_ai_context(app, domain, data_dir)?;

  let store = SiteStore::load(app)?;
  let site = store
    .get_site(domain)
    .ok_or_else(|| format!("Site not found: {domain}"))?;

  let site_path = std::path::Path::new(&site.path);

  // Write .claude/orbit-context.md
  let claude_dir = site_path.join(".claude");
  std::fs::create_dir_all(&claude_dir)
    .map_err(|e| format!("Failed to create .claude/ dir: {e}"))?;
  std::fs::write(claude_dir.join("orbit-context.md"), &content)
    .map_err(|e| format!("Failed to write .claude/orbit-context.md: {e}"))?;

  // Write .gemini/orbit-context.md
  let gemini_dir = site_path.join(".gemini");
  std::fs::create_dir_all(&gemini_dir)
    .map_err(|e| format!("Failed to create .gemini/ dir: {e}"))?;
  std::fs::write(gemini_dir.join("orbit-context.md"), &content)
    .map_err(|e| format!("Failed to write .gemini/orbit-context.md: {e}"))?;

  Ok(format!(
    "Context files written for {domain}:\n  {}\n  {}",
    claude_dir.join("orbit-context.md").display(),
    gemini_dir.join("orbit-context.md").display()
  ))
}

/// Ensure orbit-mcp is registered in Claude Code's MCP config (~/.claude.json)
pub fn setup_mcp_for_claude(app: &AppHandle) -> Result<(), String> {
  let mcp_exe = app
    .path()
    .app_local_data_dir()
    .map_err(|e| e.to_string())?
    .join("bin")
    .join("mcp")
    .join("orbit-mcp.exe");

  if !mcp_exe.exists() {
    return Err(
      "orbit-mcp is not installed. Install it from the MCP tab first.".to_string(),
    );
  }

  let mcp_path = mcp_exe.to_string_lossy().to_string().replace('\\', "/");

  // Read or create ~/.claude.json
  let home = dirs::home_dir().ok_or("Cannot find home directory")?;
  let claude_config = home.join(".claude.json");

  let mut config: serde_json::Value = if claude_config.exists() {
    let data = std::fs::read_to_string(&claude_config).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).unwrap_or(serde_json::json!({}))
  } else {
    serde_json::json!({})
  };

  // Add orbit MCP server entry if not already present
  let mcp_servers = config
    .as_object_mut()
    .ok_or("Invalid config format")?
    .entry("mcpServers")
    .or_insert(serde_json::json!({}));

  if let Some(servers) = mcp_servers.as_object_mut() {
    if !servers.contains_key("orbit") {
      servers.insert(
        "orbit".to_string(),
        serde_json::json!({
          "command": mcp_path,
          "args": []
        }),
      );
    }
  }

  std::fs::write(
    &claude_config,
    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
  )
  .map_err(|e| e.to_string())?;

  Ok(())
}

pub struct GeminiCliManager;

impl GeminiCliManager {
  /// Get npm executable path (Orbit's own Node.js)
  pub fn get_npm_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
      .path()
      .app_local_data_dir()
      .map_err(|e| e.to_string())?
      .join("bin")
      .join("nodejs");

    #[cfg(target_os = "windows")]
    return Ok(base.join("npm.cmd"));

    #[cfg(not(target_os = "windows"))]
    return Ok(base.join("bin").join("npm"));
  }

  /// Get gemini executable path in Orbit's nodejs folder
  pub fn get_orbit_exe_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
      .path()
      .app_local_data_dir()
      .map_err(|e| e.to_string())?
      .join("bin")
      .join("nodejs");

    #[cfg(target_os = "windows")]
    return Ok(base.join("gemini.cmd"));

    #[cfg(not(target_os = "windows"))]
    return Ok(base.join("bin").join("gemini"));
  }

  /// Find gemini in system PATH (outside Orbit)
  fn find_system_exe() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let cmd = "which";

    let output = hidden_command(cmd)
      .args(["gemini"])
      .output()
      .ok()?;

    if output.status.success() {
      let stdout = String::from_utf8_lossy(&output.stdout);
      let first_line = stdout.lines().next()?.trim().to_string();
      if !first_line.is_empty() {
        return Some(PathBuf::from(first_line));
      }
    }
    None
  }

  /// Get version from a specific exe path
  fn get_version_from(exe: &Path) -> Option<String> {
    let output = hidden_command(exe)
      .args(["--version"])
      .output()
      .ok()?;

    if output.status.success() {
      let stdout = String::from_utf8_lossy(&output.stdout);
      let version = stdout.trim().to_string();
      if !version.is_empty() {
        return Some(version);
      }
    }
    None
  }

  /// Get full Gemini CLI status — checks Orbit first, then system PATH
  pub fn get_status(app: &AppHandle) -> Result<AiToolStatus, String> {
    // 1. Check Orbit's own installation
    let orbit_exe = Self::get_orbit_exe_path(app)?;
    if orbit_exe.exists() {
      let version = Self::get_version_from(&orbit_exe);
      return Ok(AiToolStatus {
        installed: true,
        path: Some(orbit_exe.to_string_lossy().to_string()),
        version,
        source: Some("orbit".to_string()),
      });
    }

    // 2. Check system PATH
    if let Some(system_exe) = Self::find_system_exe() {
      let version = Self::get_version_from(&system_exe);
      return Ok(AiToolStatus {
        installed: true,
        path: Some(system_exe.to_string_lossy().to_string()),
        version,
        source: Some("system".to_string()),
      });
    }

    // 3. Not found anywhere
    Ok(AiToolStatus::default())
  }

  /// Install Gemini CLI via npm
  pub fn install(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed. Please install Node.js first.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["install", "-g", "@google/gemini-cli"])
      .output()
      .map_err(|e| format!("Failed to run npm install: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }

  /// Uninstall Gemini CLI via npm
  pub fn uninstall(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["uninstall", "-g", "@google/gemini-cli"])
      .output()
      .map_err(|e| format!("Failed to run npm uninstall: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }

  /// Update Gemini CLI via npm
  pub fn update(app: &AppHandle) -> Result<(), String> {
    let npm = Self::get_npm_path(app)?;

    if !npm.exists() {
      return Err("Node.js is not installed.".to_string());
    }

    let output = hidden_command(&npm)
      .args(["update", "-g", "@google/gemini-cli"])
      .output()
      .map_err(|e| format!("Failed to run npm update: {e}"))?;

    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      Err(format!("{stdout}\n{stderr}"))
    }
  }
}
