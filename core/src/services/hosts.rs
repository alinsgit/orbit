use crate::services::validation::{validate_domain, sanitize_for_powershell};
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
        // Validate domain before any operation
        validate_domain(domain).map_err(|e| e.to_string())?;

        let entry = format!("\n127.0.0.1 {domain}\n");

        // Read current content
        let content = fs::read_to_string(HOSTS_PATH)
            .map_err(|e| format!("Failed to read hosts file: {e}"))?;

        if content.contains(&format!("127.0.0.1 {domain}")) {
            return Ok(()); // Already exists
        }

        // Append new domain
        let mut file = fs::OpenOptions::new()
            
            .append(true)
            .open(HOSTS_PATH)
            .map_err(|e| format!("Failed to open hosts file (Permission denied?): {e}"))?;

        file.write_all(entry.as_bytes())
            .map_err(|e| format!("Failed to write to hosts file: {e}"))?;

        Ok(())
    }

    /// Add domain using elevated PowerShell (triggers UAC prompt)
    #[cfg(target_os = "windows")]
    pub fn add_domain_elevated(domain: &str) -> Result<(), String> {
        // Validate domain before any operation - CRITICAL for security
        validate_domain(domain).map_err(|e| e.to_string())?;

        // Sanitize for PowerShell (extra safety layer)
        let safe_domain = sanitize_for_powershell(domain);

        // Check if already exists
        let content = fs::read_to_string(HOSTS_PATH)
            .map_err(|e| format!("Failed to read hosts file: {e}"))?;

        if content.contains(&format!("127.0.0.1 {safe_domain}")) {
            return Ok(()); // Already exists
        }

        // Create a temporary PowerShell script with secure random name
        let temp_dir = std::env::temp_dir();
        let random_suffix: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let script_path = temp_dir.join(format!("orbit_host_{random_suffix}.ps1"));

        // Use here-string to avoid injection
        let script_content = format!(
            r#"$hostsPath = @'
{HOSTS_PATH}
'@
$domain = @'
{safe_domain}
'@
$entry = "`r`n127.0.0.1 $domain"
Add-Content -Path $hostsPath -Value $entry -Force -Encoding ASCII
"#
        );

        fs::write(&script_path, &script_content)
            .map_err(|e| format!("Failed to create temp script: {e}"))?;

        // Run the script with elevation (hidden window — only UAC prompt visible)
        let mut ps_command = Command::new("powershell");
        ps_command.args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-WindowStyle", "Hidden",
            "-Command",
            &format!(
                "Start-Process powershell -Verb RunAs -WindowStyle Hidden -Wait -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-WindowStyle', 'Hidden', '-File', '{}'",
                script_path.display()
            ),
        ]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            ps_command.creation_flags(CREATE_NO_WINDOW);
        }
        let output = ps_command.output()
            .map_err(|e| format!("Failed to execute PowerShell: {e}"))?;

        // Clean up temp script immediately
        if let Err(e) = fs::remove_file(&script_path) {
            log::warn!("Failed to remove temp script: {e}");
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("canceled") || stderr.contains("denied") {
                return Err("User cancelled UAC prompt".to_string());
            }
            return Err(format!("Failed to add domain: {stderr}"));
        }

        // Verify it was added
        let new_content = fs::read_to_string(HOSTS_PATH)
            .map_err(|e| format!("Failed to verify: {e}"))?;

        if new_content.contains(&format!("127.0.0.1 {safe_domain}")) {
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
        // Validate domain before any operation
        validate_domain(domain).map_err(|e| e.to_string())?;

        let content = fs::read_to_string(HOSTS_PATH)
            .map_err(|e| format!("Failed to read hosts file: {e}"))?;

        let entry = format!("127.0.0.1 {domain}");
        let new_content = content.replace(&entry, "").trim().to_string();

        fs::write(HOSTS_PATH, new_content)
            .map_err(|e| format!("Failed to write to hosts file: {e}"))?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub fn check_admin() -> bool {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = Command::new("net")
            .args(["session"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_domain_validation() {
        // empty domain
        let res = HostsManager::add_domain("");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Domain cannot be empty"));

        // path traversal
        let res = HostsManager::add_domain("../evil");
        assert!(res.is_err());
        
        // command injection
        let res = HostsManager::add_domain("test; rm -rf /");
        assert!(res.is_err());
    }

    #[test]
    fn test_add_domain_elevated_validation() {
        #[cfg(target_os = "windows")]
        {
            // empty domain
            let res = HostsManager::add_domain_elevated("");
            assert!(res.is_err());

            // path traversal
            let res = HostsManager::add_domain_elevated("../evil");
            assert!(res.is_err());

            // command injection
            let res = HostsManager::add_domain_elevated("test; rm -rf /");
            assert!(res.is_err());
        }
    }

    #[test]
    fn test_remove_domain_validation() {
        // empty domain
        let res = HostsManager::remove_domain("");
        assert!(res.is_err());

        // path traversal
        let res = HostsManager::remove_domain("../evil");
        assert!(res.is_err());

        // command injection
        let res = HostsManager::remove_domain("test; rm -rf /");
        assert!(res.is_err());
    }

    #[test]
    fn test_check_admin_returns_bool() {
        // We just ensure it doesn't panic and returns a boolean
        let _admin = HostsManager::check_admin();
    }
}
