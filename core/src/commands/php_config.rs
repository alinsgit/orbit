use crate::services::validation::{validate_ini_key, validate_ini_value, validate_php_version};
use std::collections::HashMap;
use std::fs;
use tauri::{command, AppHandle, Manager};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PhpExtension {
    pub name: String,
    pub enabled: bool,
    pub available: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PhpConfig {
    pub version: String,
    pub path: String,
    pub extensions: Vec<PhpExtension>,
    pub settings: HashMap<String, String>,
}

/// Get PHP configuration for a specific version
#[command]
pub fn get_php_config(app: AppHandle, version: String) -> Result<PhpConfig, String> {
    // Validate PHP version format
    validate_php_version(&version).map_err(|e| e.to_string())?;

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(&version);

    if !bin_path.exists() {
        return Err(format!("PHP {} not found", version));
    }

    let ini_path = bin_path.join("php.ini");
    let ext_dir = bin_path.join("ext");

    // Read php.ini
    let ini_content = if ini_path.exists() {
        fs::read_to_string(&ini_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Get available extensions from ext directory
    let mut extensions: Vec<PhpExtension> = Vec::new();
    if ext_dir.exists() {
        if let Ok(entries) = fs::read_dir(&ext_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("php_") && name.ends_with(".dll") {
                    let ext_name = name
                        .strip_prefix("php_")
                        .and_then(|s| s.strip_suffix(".dll"))
                        .unwrap_or(&name)
                        .to_string();

                    let enabled = ini_content.contains(&format!("extension={}", ext_name))
                        && !ini_content.contains(&format!(";extension={}", ext_name));

                    extensions.push(PhpExtension {
                        name: ext_name,
                        enabled,
                        available: true,
                    });
                }
            }
        }
    }

    // Sort extensions alphabetically
    extensions.sort_by(|a, b| a.name.cmp(&b.name));

    // Parse common settings
    let mut settings: HashMap<String, String> = HashMap::new();
    let setting_keys = [
        "memory_limit",
        "upload_max_filesize",
        "post_max_size",
        "max_execution_time",
        "max_input_time",
        "display_errors",
        "error_reporting",
        "date.timezone",
    ];

    for key in setting_keys {
        if let Some(value) = parse_ini_value(&ini_content, key) {
            settings.insert(key.to_string(), value);
        }
    }

    Ok(PhpConfig {
        version,
        path: bin_path.to_string_lossy().to_string(),
        extensions,
        settings,
    })
}

/// Validate extension name (only alphanumeric and underscore)
fn validate_extension_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Extension name cannot be empty".to_string());
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Extension name can only contain alphanumeric characters and underscores".to_string());
    }
    if name.len() > 50 {
        return Err("Extension name too long".to_string());
    }
    Ok(())
}

/// Update PHP extension status
#[command]
pub fn set_php_extension(
    app: AppHandle,
    version: String,
    extension: String,
    enabled: bool,
) -> Result<String, String> {
    // Validate inputs
    validate_php_version(&version).map_err(|e| e.to_string())?;
    validate_extension_name(&extension)?;

    let ini_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(&version)
        .join("php.ini");

    if !ini_path.exists() {
        return Err("php.ini not found".to_string());
    }

    let mut content = fs::read_to_string(&ini_path)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    let disabled = format!(";extension={}", extension);
    let enabled_str = format!("extension={}", extension);

    if enabled {
        // Enable extension
        if content.contains(&disabled) {
            content = content.replace(&disabled, &enabled_str);
        } else if !content.contains(&enabled_str) {
            // Add if not present
            content = format!("{}\n{}", content, enabled_str);
        }
    } else {
        // Disable extension
        if content.contains(&enabled_str) && !content.contains(&disabled) {
            content = content.replace(&enabled_str, &disabled);
        }
    }

    fs::write(&ini_path, content).map_err(|e| format!("Failed to write php.ini: {}", e))?;

    Ok(format!(
        "Extension {} {}",
        extension,
        if enabled { "enabled" } else { "disabled" }
    ))
}

/// Update PHP setting
#[command]
pub fn set_php_setting(
    app: AppHandle,
    version: String,
    key: String,
    value: String,
) -> Result<String, String> {
    // Validate all inputs
    validate_php_version(&version).map_err(|e| e.to_string())?;
    validate_ini_key(&key).map_err(|e| e.to_string())?;
    validate_ini_value(&value).map_err(|e| e.to_string())?;

    let ini_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin")
        .join("php")
        .join(&version)
        .join("php.ini");

    if !ini_path.exists() {
        return Err("php.ini not found".to_string());
    }

    let content = fs::read_to_string(&ini_path)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    let new_content = update_ini_value(&content, &key, &value);

    fs::write(&ini_path, new_content).map_err(|e| format!("Failed to write php.ini: {}", e))?;

    Ok(format!("Setting {} updated to {}", key, value))
}

/// Parse a value from ini content
fn parse_ini_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Update a value in ini content
fn update_ini_value(content: &str, key: &str, value: &str) -> String {
    let mut found = false;
    let mut result: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for commented version
        if trimmed.starts_with(';') {
            if let Some((k, _)) = trimmed[1..].trim().split_once('=') {
                if k.trim() == key {
                    // Replace commented line with active one
                    result.push(format!("{} = {}", key, value));
                    found = true;
                    continue;
                }
            }
        }

        // Check for active version
        if let Some((k, _)) = trimmed.split_once('=') {
            if k.trim() == key {
                result.push(format!("{} = {}", key, value));
                found = true;
                continue;
            }
        }

        result.push(line.to_string());
    }

    // Add if not found
    if !found {
        result.push(format!("{} = {}", key, value));
    }

    result.join("\n")
}
