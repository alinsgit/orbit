use tauri::{command, AppHandle, Manager};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use crate::services::hidden_command;

// ── Service directory resolution (cross-platform) ───────────────────────────

/// Resolve the bin directory for a given service type.
/// On Unix, PHP lives under `bin/<ver>/bin/`; on Windows it's flat `bin/<ver>/`.
fn service_dir(bin_path: &Path, service_type: &str) -> Option<PathBuf> {
    match service_type {
        "nginx" => Some(bin_path.join("nginx")),
        "mariadb" => {
            let b = bin_path.join("mariadb").join("bin");
            Some(if b.exists() { b } else { bin_path.join("mariadb") })
        }
        s if s.starts_with("php") => {
            let ver = s.strip_prefix("php-").unwrap_or("8.4");
            let base = bin_path.join("php").join(ver);
            #[cfg(not(windows))]
            let path = base.join("bin"); // our tar.gz extracts bin/php into <ver>/bin/
            #[cfg(windows)]
            let path = base;             // Windows zip is flat
            Some(path)
        }
        "apache" => {
            let b = bin_path.join("apache").join("bin");
            Some(if b.exists() { b } else { bin_path.join("apache") })
        }
        "redis"    => Some(bin_path.join("redis")),
        "postgresql" => {
            let pg = bin_path.join("postgresql");
            let b  = pg.join("pgsql").join("bin");
            Some(if b.exists() { b } else { pg.join("bin") })
        }
        "mongodb" => {
            let b = bin_path.join("mongodb").join("bin");
            Some(if b.exists() { b } else { bin_path.join("mongodb") })
        }
        "nodejs"   => Some(bin_path.join("nodejs")),
        "python"   => Some(bin_path.join("python")),
        "bun"      => Some(bin_path.join("bun")),
        "go"       => Some(bin_path.join("go").join("bin")),
        "deno"     => Some(bin_path.join("deno")),
        "composer" => Some(bin_path.join("composer")),
        "rust"     => Some(bin_path.join("rust")),
        "mailpit"  => Some(bin_path.join("mailpit")),
        "mcp"      => Some(bin_path.join("mcp")),
        "cli"      => Some(bin_path.join("cli")),
        _          => None,
    }
}

// ── Windows helpers ─────────────────────────────────────────────────────────

#[cfg(windows)]
fn run_ps(script: &str) -> Result<String, String> {
    let out = hidden_command("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(windows)]
fn win_add(path: &str) -> Result<String, String> {
    run_ps(&format!(r#"
        $p   = [Environment]::GetEnvironmentVariable('Path', 'User')
        $add = '{}'
        $arr = $p -split ';'
        if ($arr -notcontains $add) {{
            [Environment]::SetEnvironmentVariable('Path', ($arr + $add) -join ';', 'User')
            Write-Output "Added to PATH"
        }} else {{ Write-Output "Already in PATH" }}
    "#, path.replace("'", "''")))
}

#[cfg(windows)]
fn win_remove(pattern: &str) -> Result<String, String> {
    run_ps(&format!(r#"
        $p   = [Environment]::GetEnvironmentVariable('Path', 'User')
        $pat = '{}'
        $arr = $p -split ';'
        $filtered = $arr | Where-Object {{ $_ -ne $pat -and -not ($_ -like "$pat\*") }}
        $removed  = $arr.Count - $filtered.Count
        if ($removed -gt 0) {{
            [Environment]::SetEnvironmentVariable('Path', ($filtered -join ';'), 'User')
            Write-Output "Removed from PATH"
        }} else {{ Write-Output "Not in PATH" }}
    "#, pattern.replace("'", "''")))
}

#[cfg(windows)]
fn win_get_user_path() -> Result<Vec<String>, String> {
    let raw = run_ps("[Environment]::GetEnvironmentVariable('Path', 'User')")?;
    Ok(raw.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

#[cfg(windows)]
fn win_is_in_path(path: &str) -> bool {
    win_get_user_path().map(|paths| {
        let lower = path.to_lowercase();
        paths.iter().any(|p| {
            let pl = p.to_lowercase();
            pl == lower || pl.starts_with(&format!("{lower}\\"))
        })
    }).unwrap_or(false)
}

// ── Unix helpers ────────────────────────────────────────────────────────────

#[cfg(not(windows))]
fn home_dir() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Shell RC files we manage PATH entries in.
#[cfg(not(windows))]
fn rc_files() -> Vec<PathBuf> {
    let home = home_dir();
    [".bashrc", ".zshrc", ".bash_profile", ".profile"]
        .iter()
        .map(|f| home.join(f))
        // Only include files that already exist (don't create e.g. .zshrc on bash-only systems)
        .filter(|p| p.exists())
        .collect()
}

/// Unique comment marker for orbit-managed PATH entries.
#[cfg(not(windows))]
fn orbit_marker(service: &str) -> String {
    format!("# orbit-path:{service}")
}

/// Append `export PATH="<path>:$PATH"` to shell RC files (idempotent).
#[cfg(not(windows))]
fn unix_add(path_str: &str, service: &str) -> Result<String, String> {
    use std::io::Write;
    let marker  = orbit_marker(service);
    let line    = format!("export PATH=\"{path_str}:$PATH\"  {marker}");
    let mut added_to = vec![];

    for rc in rc_files() {
        let content = std::fs::read_to_string(&rc).unwrap_or_default();
        if content.contains(&marker) {
            continue; // already present
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true).append(true).open(&rc)
            .map_err(|e| format!("Cannot write {}: {e}", rc.display()))?;
        writeln!(f, "\n{line}")
            .map_err(|e| format!("Write error {}: {e}", rc.display()))?;
        added_to.push(rc.to_string_lossy().to_string());
    }

    if added_to.is_empty() {
        Ok("Already in PATH config".to_string())
    } else {
        Ok("Added — restart your terminal or run: source ~/.bashrc".to_string())
    }
}

/// Remove orbit-managed PATH lines for `service` from RC files.
#[cfg(not(windows))]
fn unix_remove(service: &str) -> Result<String, String> {
    let marker  = orbit_marker(service);
    let mut removed = false;

    for rc in rc_files() {
        if let Ok(content) = std::fs::read_to_string(&rc) {
            if !content.contains(&marker) {
                continue;
            }
            // Filter lines that carry our marker (including the blank line before)
            let new: String = content.lines()
                .filter(|l| !l.contains(&marker))
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(&rc, new)
                .map_err(|e| format!("Cannot write {}: {e}", rc.display()))?;
            removed = true;
        }
    }

    Ok(if removed { "Removed from PATH config".to_string() } else { "Not found in PATH config".to_string() })
}

/// Check if a path is present in the orbit-managed RC files.
#[cfg(not(windows))]
fn unix_is_in_path_rc(service: &str) -> bool {
    let marker = orbit_marker(service);
    rc_files().iter().any(|rc| {
        std::fs::read_to_string(rc).map(|c| c.contains(&marker)).unwrap_or(false)
    })
}

/// Check if a path is currently active in $PATH (current session).
#[cfg(not(windows))]
fn unix_is_in_current_path(path_str: &str) -> bool {
    std::env::var("PATH")
        .map(|p| p.split(':').any(|e| e == path_str))
        .unwrap_or(false)
}

// ── Tauri commands ──────────────────────────────────────────────────────────

/// Add a specific service to the user PATH.
#[command]
pub fn add_service_to_path(app: AppHandle, service_type: String) -> Result<String, String> {
    let bin_path = app.path().app_local_data_dir()
        .map_err(|e| e.to_string())?.join("bin");

    if !bin_path.exists() {
        return Err("Bin directory does not exist".to_string());
    }

    let svc_path = service_dir(&bin_path, &service_type)
        .ok_or_else(|| format!("Unknown service type: {service_type}"))?;

    if !svc_path.exists() {
        return Err(format!("Service path does not exist: {}", svc_path.display()));
    }

    let path_str = svc_path.to_string_lossy().to_string();

    #[cfg(windows)]
    return win_add(&path_str).map(|msg| format!("{path_str}: {msg}"));

    #[cfg(not(windows))]
    return unix_add(&path_str, &service_type).map(|msg| format!("{path_str}: {msg}"));
}

/// Remove a specific service from the user PATH.
#[command]
pub fn remove_service_from_path(app: AppHandle, service_type: String) -> Result<String, String> {
    let bin_path = app.path().app_local_data_dir()
        .map_err(|e| e.to_string())?.join("bin");

    let svc_path = service_dir(&bin_path, &service_type)
        .ok_or_else(|| format!("Unknown service type: {service_type}"))?;

    let path_str = svc_path.to_string_lossy().to_string();

    #[cfg(windows)]
    return win_remove(&path_str).map(|msg| format!("{path_str}: {msg}"));

    #[cfg(not(windows))]
    return unix_remove(&service_type).map(|msg| format!("{path_str}: {msg}"));
}

/// Check if a specific service directory is in the user PATH.
#[command]
pub fn check_service_path_status(app: AppHandle, service_type: String) -> Result<ServicePathStatus, String> {
    let bin_path = app.path().app_local_data_dir()
        .map_err(|e| e.to_string())?.join("bin");

    let svc_path = service_dir(&bin_path, &service_type)
        .ok_or_else(|| format!("Unknown service type: {service_type}"))?;

    let path_str = svc_path.to_string_lossy().to_string();

    #[cfg(windows)]
    let in_path = win_is_in_path(&path_str);

    #[cfg(not(windows))]
    let in_path = unix_is_in_path_rc(&service_type) || unix_is_in_current_path(&path_str);

    Ok(ServicePathStatus { in_path, service_path: path_str, service_type })
}

/// Add all installed services to the user PATH.
#[command]
pub fn add_to_path(app: AppHandle) -> Result<String, String> {
    let bin_path = app.path().app_local_data_dir()
        .map_err(|e| e.to_string())?.join("bin");

    if !bin_path.exists() {
        return Err("Bin directory does not exist".to_string());
    }

    let services = [
        "nginx", "mariadb", "nodejs", "python", "bun", "go",
        "deno", "composer", "rust", "mailpit", "mcp", "cli",
    ];

    let mut added: Vec<String> = vec![];

    // Fixed services
    for svc in &services {
        if let Some(p) = service_dir(&bin_path, svc) {
            if p.exists() {
                #[cfg(windows)]
                win_add(&p.to_string_lossy()).ok();
                #[cfg(not(windows))]
                unix_add(&p.to_string_lossy(), svc).ok();
                added.push(svc.to_string());
            }
        }
    }

    // PHP versions (enumerate installed)
    let php_root = bin_path.join("php");
    if php_root.exists() {
        if let Ok(entries) = std::fs::read_dir(&php_root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if let Some(ver) = entry.file_name().to_str() {
                        let svc_type = format!("php-{ver}");
                        if let Some(p) = service_dir(&bin_path, &svc_type) {
                            if p.exists() {
                                #[cfg(windows)]
                                win_add(&p.to_string_lossy()).ok();
                                #[cfg(not(windows))]
                                unix_add(&p.to_string_lossy(), &svc_type).ok();
                                added.push(svc_type);
                            }
                        }
                    }
                }
            }
        }
    }

    if added.is_empty() {
        Err("No installed services found to add".to_string())
    } else {
        Ok(format!("PATH updated for: {}", added.join(", ")))
    }
}

/// Check if orbit bin dirs are in PATH (legacy bulk check).
#[command]
pub fn check_path_status(app: AppHandle) -> Result<PathStatus, String> {
    let bin_path = app.path().app_local_data_dir()
        .map_err(|e| e.to_string())?.join("bin");
    let bin_str  = bin_path.to_string_lossy().to_string();

    #[cfg(windows)]
    let in_path = {
        let user_path = win_get_user_path().unwrap_or_default().join(";").to_lowercase();
        user_path.contains(&bin_str.to_lowercase())
    };

    #[cfg(not(windows))]
    let in_path = {
        let env_path = std::env::var("PATH").unwrap_or_default();
        env_path.split(':').any(|p| p.starts_with(&bin_str))
    };

    Ok(PathStatus { in_path, bin_dir: bin_str })
}

/// Remove all orbit bin dirs from PATH (legacy bulk remove).
#[command]
#[allow(clippy::needless_return)]
pub fn remove_from_path(app: AppHandle) -> Result<String, String> {
    let bin_path = app.path().app_local_data_dir()
        .map_err(|e| e.to_string())?.join("bin");
    #[cfg(windows)]
    let bin_str = bin_path.to_string_lossy().to_string();

    #[cfg(windows)]
    {
        let ps = format!(r#"
            $p   = [Environment]::GetEnvironmentVariable('Path', 'User')
            $bin = '{}'
            $arr = $p -split ';'
            $filtered = $arr | Where-Object {{ -not ($_ -like "$bin*") }}
            $removed  = $arr.Count - $filtered.Count
            if ($removed -gt 0) {{
                [Environment]::SetEnvironmentVariable('Path', ($filtered -join ';'), 'User')
                Write-Output "Removed $removed path(s)"
            }} else {{ Write-Output "No paths to remove" }}
        "#, bin_str.replace("'", "''"));
        return run_ps(&ps).map(|msg| format!("PATH cleaned. {msg}"));
    }

    #[cfg(not(windows))]
    {
        // Remove all orbit-managed lines from RC files
        let mut total_removed = 0usize;
        for rc in rc_files() {
            if let Ok(content) = std::fs::read_to_string(&rc) {
                let new: String = content.lines()
                    .filter(|l| !l.contains("# orbit-path:"))
                    .collect::<Vec<_>>()
                    .join("\n");
                if new.len() != content.len() {
                    std::fs::write(&rc, new).ok();
                    total_removed += 1;
                }
            }
        }
        Ok(format!("Removed orbit PATH entries from {total_removed} shell config file(s)"))
    }
}

/// Return the user PATH as a list of directory strings.
#[command]
pub fn get_user_path() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    return win_get_user_path();

    #[cfg(not(windows))]
    {
        // Return current session PATH entries
        let path = std::env::var("PATH").map_err(|_| "PATH not set".to_string())?;
        Ok(path.split(':').map(|s| s.to_string()).filter(|s| !s.is_empty()).collect())
    }
}

/// Overwrite the user PATH with the provided list of directories.
#[command]
#[allow(clippy::needless_return)]
pub fn save_user_path(paths: Vec<String>) -> Result<String, String> {
    let clean: Vec<String> = paths.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    #[cfg(windows)]
    {
        let new_path = clean.join(";");
        let ps = format!(r#"
            [Environment]::SetEnvironmentVariable('Path', '{}', 'User')
            Write-Output "User PATH updated successfully."
        "#, new_path.replace("'", "''"));
        return run_ps(&ps).map(|_| "PATH saved successfully.".to_string());
    }

    #[cfg(not(windows))]
    {
        // On Unix, PATH is managed via shell RC files.
        // We can't directly set "user PATH" like Windows registry.
        // Return an informative message instead.
        let _ = clean;
        Err("On Linux/macOS, PATH is managed through shell config files (~/.bashrc, ~/.zshrc). Use add_service_to_path instead.".to_string())
    }
}

// ── Structs ─────────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct PathStatus {
    pub in_path: bool,
    pub bin_dir: String,
}

#[derive(serde::Serialize)]
pub struct ServicePathStatus {
    pub in_path: bool,
    pub service_path: String,
    pub service_type: String,
}
