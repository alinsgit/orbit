use crate::services::hosts::HostsManager;
use tauri::command;

#[command]
pub fn add_host(domain: String) -> Result<String, String> {
    match HostsManager::add_domain(&domain) {
        Ok(_) => Ok(format!("Domain {domain} added to hosts file")),
        Err(e) => Err(e),
    }
}

#[command]
pub fn add_host_elevated(domain: String) -> Result<String, String> {
    match HostsManager::add_domain_elevated(&domain) {
        Ok(_) => Ok(format!("Domain {domain} added to hosts file")),
        Err(e) => Err(e),
    }
}

#[command]
pub fn remove_host(domain: String) -> Result<String, String> {
    match HostsManager::remove_domain(&domain) {
        Ok(_) => Ok(format!("Domain {domain} removed from hosts file")),
        Err(e) => Err(e),
    }
}

#[command]
#[allow(dead_code)]
pub fn check_admin() -> bool {
    HostsManager::check_admin()
}

/// Retrieve the raw contents of the system hosts file
#[command]
pub fn get_hosts_file() -> Result<String, String> {
    let hosts_path = if cfg!(windows) {
        std::path::Path::new(r"C:\Windows\System32\drivers\etc\hosts")
    } else {
        std::path::Path::new("/etc/hosts")
    };

    if !hosts_path.exists() {
        return Err("Hosts file does not exist on this system.".to_string());
    }

    std::fs::read_to_string(hosts_path).map_err(|e| format!("Failed to read hosts file: {e}"))
}

/// Save the new contents to the system hosts file requiring elevation
#[command]
pub fn save_hosts_file(app: tauri::AppHandle, new_content: String) -> Result<String, String> {
    use tauri::Manager;
    let hosts_path = if cfg!(windows) {
        std::path::Path::new(r"C:\Windows\System32\drivers\etc\hosts")
    } else {
        std::path::Path::new("/etc/hosts")
    };

    // 1. Write the new content to a temporary file in our app data dir
    let app_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
        
    let temp_hosts_path = app_dir.join("temp_hosts.txt");
    std::fs::write(&temp_hosts_path, new_content)
        .map_err(|e| format!("Failed to write temporary hosts file: {e}"))?;

    // 2. Execute elevated command to overwrite system hosts with temp file
    if cfg!(windows) {
        let hosts_path_str = hosts_path.to_string_lossy();
        let temp_hosts_path_str = temp_hosts_path.to_string_lossy();
        
        let temp_dir = std::env::temp_dir();
        let random_suffix: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let script_path = temp_dir.join(format!("orbit_save_hosts_{random_suffix}.ps1"));

        let script_content = format!(
            "Copy-Item -Path '{}' -Destination '{}' -Force",
            temp_hosts_path_str.replace("'", "''"), hosts_path_str.replace("'", "''")
        );

        std::fs::write(&script_path, &script_content)
            .map_err(|e| format!("Failed to create temp script: {e}"))?;

        let output = crate::services::hidden_command("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy", "Bypass",
                "-WindowStyle", "Hidden",
                "-Command",
                &format!(
                    "Start-Process powershell -Verb RunAs -WindowStyle Hidden -Wait -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-WindowStyle', 'Hidden', '-File', '{}'",
                    script_path.display()
                )
            ])
            .output()
            .map_err(|e| format!("Failed to request elevation: {e}"))?;

        let _ = std::fs::remove_file(&script_path);

        if !output.status.success() {
            let _ = std::fs::remove_file(&temp_hosts_path); // Cleanup
            return Err("Elevation was denied or the process failed.".into());
        }
    } else {
        // Unix (Polkit / pkexec)
        let hosts_path_str = hosts_path.to_string_lossy();
        let temp_hosts_path_str = temp_hosts_path.to_string_lossy();
        
        let output = std::process::Command::new("pkexec")
            .args(["cp", &temp_hosts_path_str, &hosts_path_str])
            .output()
            .map_err(|e| format!("Failed to execute pkexec: {e}"))?;

        if !output.status.success() {
            let _ = std::fs::remove_file(&temp_hosts_path); // Cleanup
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Unix elevation failed: {stderr}"));
        }
    }

    // 3. Cleanup temp file ignoring errors
    let _ = std::fs::remove_file(temp_hosts_path);

    Ok("Hosts file updated successfully.".into())
}
