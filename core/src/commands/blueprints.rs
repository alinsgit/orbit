use crate::services::blueprints::{get_blueprints, Blueprint};
use crate::services::sites::{Site, SiteManager, SiteWithStatus};
use crate::services::site_store::SiteStore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::command;
use tauri::AppHandle;

#[derive(Serialize, Deserialize)]
pub struct BlueprintResult {
    pub site: SiteWithStatus,
    pub scaffold_commands: Vec<String>,
    pub dev_command: Option<String>,
    pub warnings: Vec<String>,
}

#[command]
pub fn list_blueprints() -> Result<Vec<Blueprint>, String> {
    Ok(get_blueprints())
}

#[command]
pub fn create_from_blueprint(
    app: AppHandle,
    blueprint: String,
    domain: String,
    path: String,
    php_version: Option<String>,
) -> Result<BlueprintResult, String> {
    let blueprints = get_blueprints();
    let bp = blueprints.iter().find(|b| b.name == blueprint)
        .ok_or_else(|| format!("Blueprint '{}' not found", blueprint))?;

    let mut warnings: Vec<String> = vec![];

    // Create project directory
    let project_path = Path::new(&path);
    if !project_path.exists() {
        fs::create_dir_all(project_path)
            .map_err(|e| format!("Failed to create project directory: {}", e))?;
    }

    // Determine PHP version for PHP-based blueprints
    let needs_php = bp.services.iter().any(|s| s == "php");
    let php_ver = if needs_php {
        php_version.clone().or_else(|| Some("8.4".into()))
    } else {
        None
    };

    // Create the site with the blueprint's template
    let site = Site {
        domain: domain.clone(),
        path: path.clone(),
        port: 80,
        php_version: php_ver,
        php_port: None,
        ssl_enabled: false,
        template: Some(bp.template.clone()),
        web_server: "nginx".into(),
        dev_port: None,
        dev_command: bp.dev_command.clone(),
    };

    let created_site = SiteManager::create_site(&app, site)?;

    // Write .env if blueprint has env_template
    if let Some(ref env_tpl) = bp.env_template {
        let db_name = domain.replace('.', "_").replace('-', "_");
        let env_content = env_tpl
            .replace("{{domain}}", &domain)
            .replace("{{db_name}}", &db_name);

        let env_path = project_path.join(".env");
        if !env_path.exists() {
            if let Err(e) = fs::write(&env_path, &env_content) {
                warnings.push(format!("Failed to write .env: {}", e));
            }
        } else {
            warnings.push(".env file already exists, skipped".into());
        }
    }

    // Set dev_command on site metadata if present
    if let Some(ref dev_cmd) = bp.dev_command {
        match SiteStore::load(&app) {
            Ok(mut store) => {
                if let Some(site_meta) = store.get_site_mut(&domain) {
                    site_meta.dev_command = Some(dev_cmd.clone());
                    site_meta.updated_at = chrono::Local::now().to_rfc3339();
                    if let Err(e) = store.save(&app) {
                        warnings.push(format!("Failed to save dev_command: {}", e));
                    }
                }
            }
            Err(e) => warnings.push(format!("Failed to update site metadata: {}", e)),
        }
    }

    Ok(BlueprintResult {
        site: created_site,
        scaffold_commands: bp.scaffold.clone(),
        dev_command: bp.dev_command.clone(),
        warnings,
    })
}
