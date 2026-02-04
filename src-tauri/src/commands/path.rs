use std::process::Command;
use tauri::{command, AppHandle, Manager};

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

/// Add a specific service to user PATH environment variable
#[command]
pub fn add_service_to_path(app: AppHandle, service_type: String) -> Result<String, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    if !bin_path.exists() {
        return Err("Bin directory does not exist".to_string());
    }

    // Get the path for this specific service
    let service_path = match service_type.as_str() {
        "nginx" => bin_path.join("nginx"),
        "mariadb" => {
            let mariadb_bin = bin_path.join("mariadb").join("bin");
            if mariadb_bin.exists() {
                mariadb_bin
            } else {
                bin_path.join("mariadb")
            }
        }
        s if s.starts_with("php") => {
            // Handle "php-8.4" format - extract version part
            let version = s.strip_prefix("php-").unwrap_or("8.4");
            // PHP is installed in bin/php/{version} (e.g., bin/php/8.4)
            bin_path.join("php").join(version)
        }
        "nodejs" => bin_path.join("nodejs"),
        "python" => bin_path.join("python"),
        "bun" => bin_path.join("bun"),
        _ => return Err(format!("Unknown service type: {}", service_type)),
    };

    if !service_path.exists() {
        return Err(format!("Service path does not exist: {}", service_path.display()));
    }

    let path_str = service_path.to_string_lossy().to_string();

    // PowerShell script to add to user PATH
    let ps_script = format!(
        r#"
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $pathToAdd = '{}'
        $existingPaths = $userPath -split ';'

        if ($existingPaths -notcontains $pathToAdd) {{
            $newUserPath = ($existingPaths + $pathToAdd) -join ';'
            [Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
            Write-Output "Added to PATH"
        }} else {{
            Write-Output "Already in PATH"
        }}
        "#,
        path_str.replace("'", "''")
    );

    let output = hidden_command("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("{}: {}", path_str, stdout.trim()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to update PATH: {}", stderr))
    }
}

/// Remove a specific service from user PATH environment variable
#[command]
pub fn remove_service_from_path(app: AppHandle, service_type: String) -> Result<String, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    // Get the path pattern for this specific service
    let path_pattern = match service_type.as_str() {
        "nginx" => bin_path.join("nginx").to_string_lossy().to_string(),
        "mariadb" => bin_path.join("mariadb").to_string_lossy().to_string(),
        s if s.starts_with("php") => {
            // Handle "php-8.4" format - extract version part
            let version = s.strip_prefix("php-").unwrap_or("8.4");
            bin_path.join("php").join(version).to_string_lossy().to_string()
        }
        "nodejs" => bin_path.join("nodejs").to_string_lossy().to_string(),
        "python" => bin_path.join("python").to_string_lossy().to_string(),
        "bun" => bin_path.join("bun").to_string_lossy().to_string(),
        _ => return Err(format!("Unknown service type: {}", service_type)),
    };

    // PowerShell script to remove from user PATH
    let ps_script = format!(
        r#"
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $pathToRemove = '{}'
        $existingPaths = $userPath -split ';'

        $filteredPaths = $existingPaths | Where-Object {{
            $_ -ne $pathToRemove -and -not ($_ -like "$pathToRemove\*")
        }}

        $removedCount = $existingPaths.Count - $filteredPaths.Count

        if ($removedCount -gt 0) {{
            $newUserPath = $filteredPaths -join ';'
            [Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
            Write-Output "Removed from PATH"
        }} else {{
            Write-Output "Not in PATH"
        }}
        "#,
        path_pattern.replace("'", "''")
    );

    let output = hidden_command("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("{}: {}", path_pattern, stdout.trim()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to update PATH: {}", stderr))
    }
}

/// Check if a specific service is in PATH
#[command]
pub fn check_service_path_status(app: AppHandle, service_type: String) -> Result<ServicePathStatus, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    // Get the path for this specific service
    let service_path = match service_type.as_str() {
        "nginx" => bin_path.join("nginx").to_string_lossy().to_string(),
        "mariadb" => {
            let mariadb_bin = bin_path.join("mariadb").join("bin");
            if mariadb_bin.exists() {
                mariadb_bin.to_string_lossy().to_string()
            } else {
                bin_path.join("mariadb").to_string_lossy().to_string()
            }
        }
        s if s.starts_with("php") => {
            // Handle "php-8.4" format - extract version part
            let version = s.strip_prefix("php-").unwrap_or("8.4");
            bin_path.join("php").join(version).to_string_lossy().to_string()
        }
        "nodejs" => bin_path.join("nodejs").to_string_lossy().to_string(),
        "python" => bin_path.join("python").to_string_lossy().to_string(),
        "bun" => bin_path.join("bun").to_string_lossy().to_string(),
        _ => return Err(format!("Unknown service type: {}", service_type)),
    };

    // Get current user PATH
    let output = hidden_command("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::GetEnvironmentVariable('Path', 'User')",
        ])
        .output()
        .map_err(|e| format!("Failed to get PATH: {}", e))?;

    let user_path = String::from_utf8_lossy(&output.stdout).to_lowercase();
    let service_path_lower = service_path.to_lowercase();

    let in_path = user_path.split(';').any(|p| {
        p.trim() == service_path_lower || p.trim().starts_with(&format!("{}\\", service_path_lower))
    });

    Ok(ServicePathStatus {
        in_path,
        service_path,
        service_type,
    })
}

/// Add all installed services to PATH (bulk operation)
#[command]
pub fn add_to_path(app: AppHandle) -> Result<String, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    if !bin_path.exists() {
        return Err("Bin directory does not exist".to_string());
    }

    // Collect all paths to add
    let mut paths_to_add: Vec<String> = Vec::new();

    // 1. Nginx
    let nginx_path = bin_path.join("nginx");
    if nginx_path.exists() {
        paths_to_add.push(nginx_path.to_string_lossy().to_string());
    }

    // 2. MariaDB bin
    let mariadb_bin = bin_path.join("mariadb").join("bin");
    if mariadb_bin.exists() {
        paths_to_add.push(mariadb_bin.to_string_lossy().to_string());
    } else {
        // Flat structure
        let mariadb_path = bin_path.join("mariadb");
        if mariadb_path.exists() {
            paths_to_add.push(mariadb_path.to_string_lossy().to_string());
        }
    }

    // 3. PHP versions
    let php_root = bin_path.join("php");
    if php_root.exists() {
        if let Ok(entries) = std::fs::read_dir(&php_root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    paths_to_add.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }

    // 4. Node.js
    let nodejs_path = bin_path.join("nodejs");
    if nodejs_path.exists() {
        paths_to_add.push(nodejs_path.to_string_lossy().to_string());
    }

    // 5. Python
    let python_path = bin_path.join("python");
    if python_path.exists() {
        paths_to_add.push(python_path.to_string_lossy().to_string());
    }

    // 6. Bun
    let bun_path = bin_path.join("bun");
    if bun_path.exists() {
        paths_to_add.push(bun_path.to_string_lossy().to_string());
    }

    if paths_to_add.is_empty() {
        return Err("No services found to add to PATH".to_string());
    }

    // Build PowerShell script to add to user PATH
    let paths_string = paths_to_add.join(";");

    let ps_script = format!(
        r#"
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $pathsToAdd = '{}'
        $newPaths = $pathsToAdd -split ';'
        $existingPaths = $userPath -split ';'

        $addedPaths = @()
        foreach ($path in $newPaths) {{
            if ($existingPaths -notcontains $path) {{
                $addedPaths += $path
            }}
        }}

        if ($addedPaths.Count -gt 0) {{
            $newUserPath = ($existingPaths + $addedPaths) -join ';'
            [Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
            Write-Output "Added $($addedPaths.Count) path(s)"
        }} else {{
            Write-Output "All paths already in PATH"
        }}
        "#,
        paths_string.replace("'", "''")
    );

    let output = hidden_command("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!(
            "PATH updated successfully. Added: {}\n{}",
            paths_to_add.join(", "),
            stdout.trim()
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to update PATH: {}", stderr))
    }
}

/// Check if services are in PATH (legacy)
#[command]
pub fn check_path_status(app: AppHandle) -> Result<PathStatus, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    // Get current user PATH
    let output = hidden_command("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::GetEnvironmentVariable('Path', 'User')",
        ])
        .output()
        .map_err(|e| format!("Failed to get PATH: {}", e))?;

    let user_path = String::from_utf8_lossy(&output.stdout).to_lowercase();
    let bin_path_str = bin_path.to_string_lossy().to_lowercase();

    let in_path = user_path.contains(&bin_path_str);

    Ok(PathStatus {
        in_path,
        bin_dir: bin_path.to_string_lossy().to_string(),
    })
}

/// Remove bin directories from user PATH environment variable (legacy)
#[command]
pub fn remove_from_path(app: AppHandle) -> Result<String, String> {
    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    let bin_path_str = bin_path.to_string_lossy().to_string();

    // PowerShell script to remove paths containing our bin directory
    let ps_script = format!(
        r#"
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $binPath = '{}'
        $existingPaths = $userPath -split ';'

        $filteredPaths = $existingPaths | Where-Object {{
            -not ($_ -like "$binPath*")
        }}

        $removedCount = $existingPaths.Count - $filteredPaths.Count

        if ($removedCount -gt 0) {{
            $newUserPath = $filteredPaths -join ';'
            [Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
            Write-Output "Removed $removedCount path(s)"
        }} else {{
            Write-Output "No paths to remove"
        }}
        "#,
        bin_path_str.replace("'", "''")
    );

    let output = hidden_command("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("PATH cleaned. {}", stdout.trim()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to update PATH: {}", stderr))
    }
}

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
