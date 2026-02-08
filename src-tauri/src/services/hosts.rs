use std::fs;
use std::io::Write;
use std::process::Command;

#[cfg(target_os = "windows")]
const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

#[cfg(not(target_os = "windows"))]
const HOSTS_PATH: &str = "/etc/hosts";

pub struct HostsManager;

impl HostsManager {
    pub fn add_domain(domain: &str) -> Result<(), String> {
        let entry = format!("\n127.0.0.1 {}\n", domain);

        // Read current content
        let content = fs::read_to_string(HOSTS_PATH)
            .map_err(|e| format!("Failed to read hosts file: {}", e))?;

        if content.contains(&format!("127.0.0.1 {}", domain)) {
            return Ok(()); // Already exists
        }

        // Append new domain
        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(HOSTS_PATH)
            .map_err(|e| format!("Failed to open hosts file (Permission denied?): {}", e))?;

        file.write_all(entry.as_bytes())
            .map_err(|e| format!("Failed to write to hosts file: {}", e))?;

        Ok(())
    }

    /// Add domain using elevated PowerShell (triggers UAC prompt)
    #[cfg(target_os = "windows")]
    pub fn add_domain_elevated(domain: &str) -> Result<(), String> {
        // Check if already exists
        let content = fs::read_to_string(HOSTS_PATH)
            .map_err(|e| format!("Failed to read hosts file: {}", e))?;

        if content.contains(&format!("127.0.0.1 {}", domain)) {
            return Ok(()); // Already exists
        }

        // Create a temporary PowerShell script to avoid escape issues
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("orbit_add_host.ps1");

        let script_content = format!(
            r#"
$hostsPath = '{}'
$entry = "`r`n127.0.0.1 {}"
Add-Content -Path $hostsPath -Value $entry -Force -Encoding ASCII
"#,
            HOSTS_PATH, domain
        );

        fs::write(&script_path, &script_content)
            .map_err(|e| format!("Failed to create temp script: {}", e))?;

        // Run the script with elevation
        let output = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Start-Process powershell -Verb RunAs -Wait -ArgumentList '-ExecutionPolicy', 'Bypass', '-File', '{}'",
                    script_path.display()
                ),
            ])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

        // Clean up temp script
        let _ = fs::remove_file(&script_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("canceled") || stderr.contains("denied") {
                return Err("User cancelled UAC prompt".to_string());
            }
            return Err(format!("Failed to add domain: {}", stderr));
        }

        // Verify it was added
        let new_content = fs::read_to_string(HOSTS_PATH)
            .map_err(|e| format!("Failed to verify: {}", e))?;

        if new_content.contains(&format!("127.0.0.1 {}", domain)) {
            Ok(())
        } else {
            Err("Domain was not added (unknown error)".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn add_domain_elevated(domain: &str) -> Result<(), String> {
        // On non-Windows, just try normal add
        Self::add_domain(domain)
    }

    pub fn remove_domain(domain: &str) -> Result<(), String> {
        let content = fs::read_to_string(HOSTS_PATH)
            .map_err(|e| format!("Failed to read hosts file: {}", e))?;

        let entry = format!("127.0.0.1 {}", domain);
        let new_content = content.replace(&entry, "").trim().to_string();

        fs::write(HOSTS_PATH, new_content)
            .map_err(|e| format!("Failed to write to hosts file: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub fn check_admin() -> bool {
        use std::process::Command;

        let output = Command::new("net").args(&["session"]).output();

        match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).contains("Administrator"),
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[allow(dead_code)]
    pub fn check_admin() -> bool {
        let output = Command::new("id").arg("-u").output();

        match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "0",
            Err(_) => false,
        }
    }
}
