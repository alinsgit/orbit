use tauri::command;

#[derive(serde::Serialize)]
pub struct SystemRequirements {
    pub vc_redist_installed: bool,
}

#[command]
pub fn check_system_requirements() -> SystemRequirements {
    let vc_redist_installed = check_vc_redist();
    
    SystemRequirements {
        vc_redist_installed,
    }
}

#[cfg(target_os = "windows")]
fn check_vc_redist() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    
    // Check for VC++ 2015-2022 Redistributable (x64)
    // Registry key: HKLM\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key_path = r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64";
    
    if let Ok(key) = hklm.open_subkey(key_path) {
        // Check "Installed" value (DWORD 1)
        if let Ok(installed) = key.get_value::<u32, _>("Installed") {
            return installed == 1;
        }
    }
    
    // Fallback: Check for older versions or x86 if needed
    false
}

#[cfg(not(target_os = "windows"))]
fn check_vc_redist() -> bool {
    // VC++ Redistributable is Windows-only, not needed on other platforms
    true
}
