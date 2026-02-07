use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::hidden_command;

/// Composer status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposerStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub php_version: Option<String>,
}

/// Composer project info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposerProject {
    pub name: Option<String>,
    pub description: Option<String>,
    pub dependencies: Vec<ComposerDependency>,
    pub dev_dependencies: Vec<ComposerDependency>,
}

/// Composer dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposerDependency {
    pub name: String,
    pub version: String,
    pub installed_version: Option<String>,
}

pub struct ComposerManager;

impl ComposerManager {
    /// Get Composer directory
    fn get_composer_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let bin_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("composer");
        Ok(bin_dir)
    }

    /// Get Composer phar path
    pub fn get_composer_path(app: &AppHandle) -> Result<PathBuf, String> {
        let composer_dir = Self::get_composer_dir(app)?;
        Ok(composer_dir.join("composer.phar"))
    }

    /// Check if Composer is installed
    pub fn is_installed(app: &AppHandle) -> Result<bool, String> {
        let composer_path = Self::get_composer_path(app)?;
        Ok(composer_path.exists())
    }

    /// Get PHP executable path
    fn get_php_exe(app: &AppHandle) -> Result<PathBuf, String> {
        let bin_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("php");

        // Find first available PHP version
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let php_exe = path.join("php.exe");
                    if php_exe.exists() {
                        return Ok(php_exe);
                    }
                }
            }
        }

        Err("No PHP installation found".to_string())
    }

    /// Get Composer version
    pub fn get_version(app: &AppHandle) -> Result<Option<String>, String> {
        if !Self::is_installed(app)? {
            return Ok(None);
        }

        let php_exe = match Self::get_php_exe(app) {
            Ok(exe) => exe,
            Err(_) => return Ok(None), // No PHP = can't detect version
        };
        let composer_path = Self::get_composer_path(app)?;

        let output = hidden_command(&php_exe)
            .args([composer_path.to_string_lossy().as_ref(), "--version", "--no-ansi"])
            .output()
            .ok();

        if let Some(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse "Composer version 2.x.x ..." - find first token that looks like a version
                for word in stdout.split_whitespace() {
                    if word.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                        && word.contains('.')
                    {
                        return Ok(Some(word.to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get full Composer status
    pub fn get_status(app: &AppHandle) -> Result<ComposerStatus, String> {
        let installed = Self::is_installed(app)?;
        let path = if installed {
            Some(Self::get_composer_path(app)?.to_string_lossy().to_string())
        } else {
            None
        };

        let version = if installed {
            Self::get_version(app)?
        } else {
            None
        };

        let php_version = if let Ok(php_exe) = Self::get_php_exe(app) {
            // Try "php -r "echo PHP_VERSION;"" - clean output, easy to parse
            // Note: PHP may output warnings (e.g. opcache) to stdout before the version
            let output = hidden_command(&php_exe)
                .args(["-r", "echo PHP_VERSION;"])
                .output()
                .ok();

            let version_from_cmd = output.and_then(|o| {
                if o.status.success() {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    // Find the line that looks like a version (X.Y.Z), skip warnings
                    for line in stdout.lines().rev() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty()
                            && trimmed.contains('.')
                            && trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                            && !trimmed.contains(' ')
                        {
                            return Some(trimmed.to_string());
                        }
                    }
                }
                None
            });

            // Fallback: extract version from directory name (bin/php/8.4/php.exe -> "8.4")
            version_from_cmd.or_else(|| {
                php_exe.parent().and_then(|dir| {
                    dir.file_name().map(|name| name.to_string_lossy().to_string())
                })
            })
        } else {
            None
        };

        Ok(ComposerStatus {
            installed,
            path,
            version,
            php_version,
        })
    }

    /// Download and install Composer
    pub async fn install(app: &AppHandle) -> Result<(), String> {
        let composer_dir = Self::get_composer_dir(app)?;

        // Create directory
        fs::create_dir_all(&composer_dir)
            .map_err(|e| format!("Failed to create Composer directory: {}", e))?;

        let composer_path = composer_dir.join("composer.phar");

        // Download Composer
        let url = "https://getcomposer.org/download/latest-stable/composer.phar";
        crate::services::download::download_file(url, &composer_path).await?;

        // Create batch wrapper for easy command line use
        let batch_content = format!(
            r#"@echo off
php "{}" %*
"#,
            composer_path.to_string_lossy()
        );

        let batch_path = composer_dir.join("composer.bat");
        fs::write(&batch_path, batch_content)
            .map_err(|e| format!("Failed to create Composer batch file: {}", e))?;

        Ok(())
    }

    /// Uninstall Composer
    pub fn uninstall(app: &AppHandle) -> Result<(), String> {
        let composer_dir = Self::get_composer_dir(app)?;

        if composer_dir.exists() {
            fs::remove_dir_all(&composer_dir)
                .map_err(|e| format!("Failed to remove Composer: {}", e))?;
        }

        Ok(())
    }

    /// Run Composer command in a project directory
    pub fn run_command(app: &AppHandle, project_path: &str, args: &[&str]) -> Result<String, String> {
        let php_exe = Self::get_php_exe(app)?;
        let composer_path = Self::get_composer_path(app)?;

        if !composer_path.exists() {
            return Err("Composer is not installed".to_string());
        }

        let mut command = hidden_command(&php_exe);
        command
            .arg(composer_path.to_string_lossy().as_ref())
            .args(args)
            .current_dir(project_path);

        let output = command
            .output()
            .map_err(|e| format!("Failed to run Composer: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(format!("{}\n{}", stdout, stderr))
        }
    }

    /// Install project dependencies
    pub fn install_dependencies(app: &AppHandle, project_path: &str) -> Result<String, String> {
        Self::run_command(app, project_path, &["install", "--no-interaction", "--no-ansi"])
    }

    /// Update project dependencies
    pub fn update_dependencies(app: &AppHandle, project_path: &str) -> Result<String, String> {
        Self::run_command(app, project_path, &["update", "--no-interaction", "--no-ansi"])
    }

    /// Require a package
    pub fn require_package(app: &AppHandle, project_path: &str, package: &str, dev: bool) -> Result<String, String> {
        let mut args = vec!["require", package, "--no-interaction", "--no-ansi"];
        if dev {
            args.push("--dev");
        }
        Self::run_command(app, project_path, &args)
    }

    /// Remove a package
    pub fn remove_package(app: &AppHandle, project_path: &str, package: &str) -> Result<String, String> {
        Self::run_command(app, project_path, &["remove", package, "--no-interaction", "--no-ansi"])
    }

    /// Get project info from composer.json
    pub fn get_project_info(project_path: &str) -> Result<ComposerProject, String> {
        let composer_json = PathBuf::from(project_path).join("composer.json");

        if !composer_json.exists() {
            return Err("composer.json not found".to_string());
        }

        let content = fs::read_to_string(&composer_json)
            .map_err(|e| format!("Failed to read composer.json: {}", e))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse composer.json: {}", e))?;

        let name = json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let description = json.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

        let dependencies = Self::parse_dependencies(json.get("require"));
        let dev_dependencies = Self::parse_dependencies(json.get("require-dev"));

        Ok(ComposerProject {
            name,
            description,
            dependencies,
            dev_dependencies,
        })
    }

    fn parse_dependencies(deps: Option<&serde_json::Value>) -> Vec<ComposerDependency> {
        let mut result = Vec::new();

        if let Some(deps) = deps {
            if let Some(obj) = deps.as_object() {
                for (name, version) in obj {
                    if name != "php" && !name.starts_with("ext-") {
                        result.push(ComposerDependency {
                            name: name.clone(),
                            version: version.as_str().unwrap_or("*").to_string(),
                            installed_version: None,
                        });
                    }
                }
            }
        }

        result
    }

    /// Self-update Composer
    pub fn self_update(app: &AppHandle) -> Result<String, String> {
        let php_exe = Self::get_php_exe(app)?;
        let composer_path = Self::get_composer_path(app)?;

        let output = hidden_command(&php_exe)
            .args([composer_path.to_string_lossy().as_ref(), "self-update", "--no-ansi"])
            .output()
            .map_err(|e| format!("Failed to update Composer: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("{}\n{}", stdout, stderr))
        }
    }
}
