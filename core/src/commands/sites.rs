use crate::services::nginx::NginxManager;
use crate::services::site_process::SiteProcessManager;
use crate::services::site_store::SiteStore;
use crate::services::sites::{Site, SiteManager, SiteWithStatus};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::command;
use tauri::AppHandle;

#[derive(Serialize, Deserialize)]
pub struct SiteExport {
    pub version: String,
    pub exported_at: String,
    pub sites: Vec<SiteExportEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct SiteExportEntry {
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub php_version: Option<String>,
    pub ssl_enabled: bool,
    pub template: Option<String>,
    pub web_server: String,
}

#[command]
pub fn create_site(app: AppHandle, site: Site) -> Result<SiteWithStatus, String> {
    SiteManager::create_site(&app, site)
}

#[command]
pub fn get_sites(app: AppHandle) -> Result<Vec<SiteWithStatus>, String> {
    SiteManager::get_sites(&app)
}

#[command]
pub fn get_site(app: AppHandle, domain: String) -> Result<Option<SiteWithStatus>, String> {
    SiteManager::get_site(&app, &domain)
}

#[command]
pub fn update_site(app: AppHandle, domain: String, site: Site) -> Result<SiteWithStatus, String> {
    SiteManager::update_site(&app, &domain, site)
}

#[command]
pub fn delete_site(app: AppHandle, domain: String) -> Result<String, String> {
    SiteManager::delete_site(&app, &domain)?;
    Ok("Site deleted successfully".to_string())
}

#[command]
pub fn regenerate_site_config(app: AppHandle, domain: String) -> Result<String, String> {
    SiteManager::regenerate_config(&app, &domain)?;
    Ok("Config regenerated successfully".to_string())
}

// Nginx management commands
#[command]
pub fn nginx_test_config(app: AppHandle) -> Result<String, String> {
    NginxManager::test_config(&app)
}

#[command]
pub fn nginx_reload(app: AppHandle) -> Result<String, String> {
    NginxManager::reload(&app)
}

#[command]
pub fn nginx_status() -> Result<bool, String> {
    Ok(NginxManager::is_running())
}

#[command]
pub fn scaffold_basic_project(path: String, template: String) -> Result<String, String> {
    let project_path = Path::new(&path);

    // Create directory
    fs::create_dir_all(project_path)
        .map_err(|e| format!("Failed to create directory: {e}"))?;

    match template.as_str() {
        "http" => {
            let index_php = project_path.join("index.php");
            fs::write(&index_php, "<?php\nphpinfo();\n")
                .map_err(|e| format!("Failed to create index.php: {e}"))?;
            Ok(format!("Created PHP project at {path}"))
        }
        "static" => {
            let index_html = project_path.join("index.html");
            let content = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My Site</title>
</head>
<body>
    <h1>Welcome</h1>
    <p>Your static site is ready.</p>
</body>
</html>"#;
            fs::write(&index_html, content)
                .map_err(|e| format!("Failed to create index.html: {e}"))?;
            Ok(format!("Created static project at {path}"))
        }
        "litecart" => {
            // Just create the directory, user downloads LiteCart manually
            Ok(format!("Created directory at {path}. Download LiteCart files into this folder."))
        }
        _ => Err(format!("Unsupported template for basic scaffold: {template}")),
    }
}

#[command]
pub fn export_sites(app: AppHandle) -> Result<SiteExport, String> {
    let sites = SiteManager::get_sites(&app)?;

    let export_entries: Vec<SiteExportEntry> = sites
        .iter()
        .map(|site| SiteExportEntry {
            name: site.domain.replace(".local", ""),
            domain: site.domain.clone(),
            root_path: site.path.clone(),
            php_version: site.php_version.clone(),
            ssl_enabled: site.ssl_enabled,
            template: site.template.clone(),
            web_server: site.web_server.clone(),
        })
        .collect();

    Ok(SiteExport {
        version: "1.0".to_string(),
        exported_at: chrono::Local::now().to_rfc3339(),
        sites: export_entries,
    })
}

#[command]
pub fn import_sites(app: AppHandle, import_data: SiteExport, skip_existing: bool) -> Result<ImportResult, String> {
    let existing_sites = SiteManager::get_sites(&app)?;
    let existing_domains: Vec<String> = existing_sites.iter().map(|s| s.domain.clone()).collect();

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors: Vec<String> = vec![];

    for entry in import_data.sites {
        // Check if site already exists
        if existing_domains.contains(&entry.domain) {
            if skip_existing {
                skipped += 1;
                continue;
            } else {
                errors.push(format!("Site {} already exists", entry.domain));
                continue;
            }
        }

        // Create site from import entry
        let site = Site {
            domain: entry.domain.clone(),
            path: entry.root_path,
            port: 80,
            php_version: entry.php_version,
            php_port: None,
            ssl_enabled: entry.ssl_enabled,
            template: entry.template,
            web_server: entry.web_server,
            dev_port: None,
            dev_command: None,
        };

        match SiteManager::create_site(&app, site) {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("Failed to import {}: {}", entry.domain, e)),
        }
    }

    Ok(ImportResult {
        imported,
        skipped,
        errors,
    })
}

#[derive(Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

// Per-site nginx config read/write

#[command]
pub fn read_site_config(app: AppHandle, domain: String) -> Result<String, String> {
    let sites_dir = NginxManager::get_sites_dir(&app)?;
    let conf_path = sites_dir.join(format!("{domain}.conf"));

    if !conf_path.exists() {
        return Err(format!("No nginx config found for '{domain}'"));
    }

    fs::read_to_string(&conf_path)
        .map_err(|e| format!("Failed to read config: {e}"))
}

#[command]
pub fn write_site_config(app: AppHandle, domain: String, content: String) -> Result<String, String> {
    let sites_dir = NginxManager::get_sites_dir(&app)?;
    let conf_path = sites_dir.join(format!("{domain}.conf"));

    if !conf_path.exists() {
        return Err(format!("No nginx config found for '{domain}'"));
    }

    // Backup old config
    let backup = fs::read_to_string(&conf_path).ok();

    // Write new config
    fs::write(&conf_path, &content)
        .map_err(|e| format!("Failed to write config: {e}"))?;

    // Validate with nginx -t
    match NginxManager::test_config(&app) {
        Ok(_) => {
            // Reload nginx if running
            if NginxManager::is_running() {
                let _ = NginxManager::reload(&app);
            }
            Ok("Config saved and validated successfully".to_string())
        }
        Err(e) => {
            // Rollback on validation failure
            if let Some(old_content) = backup {
                let _ = fs::write(&conf_path, old_content);
            }
            Err(format!("Config validation failed (rolled back): {e}"))
        }
    }
}

// Site app process management commands

#[command]
pub fn start_site_app(
    app: AppHandle,
    state: tauri::State<SiteProcessManager>,
    domain: String,
) -> Result<u32, String> {
    let store = SiteStore::load(&app)?;
    let site = store
        .get_site(&domain)
        .ok_or_else(|| format!("Site {domain} not found"))?;

    let dev_command = site
        .dev_command
        .as_ref()
        .ok_or_else(|| format!("Site {domain} has no dev_command configured"))?;

    state.start(&domain, dev_command, &site.path, site.dev_port)
}

#[command]
pub fn stop_site_app(
    state: tauri::State<SiteProcessManager>,
    domain: String,
) -> Result<(), String> {
    state.stop(&domain)
}

#[command]
pub fn get_site_app_status(
    state: tauri::State<SiteProcessManager>,
    domain: String,
) -> Result<String, String> {
    Ok(state.status(&domain))
}
