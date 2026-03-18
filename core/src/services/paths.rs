//! Shared path utilities for Orbit.
//! Used by both GUI (Tauri) and CLI to resolve data directories.
//!
//! Windows: %LOCALAPPDATA%/com.orbit.dev/
//! This matches Tauri's `app_local_data_dir()` for the "com.orbit.dev" identifier.

use std::path::PathBuf;

/// Get the Orbit data directory.
/// This must match Tauri's app_local_data_dir for "com.orbit.dev".
#[allow(dead_code)]
pub fn get_orbit_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| {
                let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
                format!("{home}\\AppData\\Local")
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

/// Get the bin directory where services are installed.
/// e.g. %LOCALAPPDATA%/com.orbit.dev/bin/
#[allow(dead_code)]
pub fn get_bin_dir() -> PathBuf {
    get_orbit_data_dir().join("bin")
}

/// Get the data directory for service runtime data.
/// e.g. %LOCALAPPDATA%/com.orbit.dev/bin/data/
#[allow(dead_code)]
pub fn get_service_data_dir() -> PathBuf {
    get_bin_dir().join("data")
}
