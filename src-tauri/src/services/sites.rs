use crate::services::apache::ApacheManager;
use crate::services::hosts::HostsManager;
use crate::services::nginx::NginxManager;
use crate::services::php_registry::PhpRegistry;
use crate::services::site_store::{SiteMetadata, SiteStore};
use crate::services::ssl::SSLManager;
use crate::services::templates::{SiteTemplate, TemplateEngine, TEMPLATE_LITECART_SSL};
use crate::services::validation::{validate_domain, validate_port, validate_site_path, sanitize_for_nginx};
use std::collections::HashMap;
use std::fs;
use tauri::{AppHandle, Manager};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Site {
    pub domain: String,
    pub path: String,
    pub port: u16,
    pub php_version: Option<String>,
    #[serde(default)]
    pub php_port: Option<u16>,
    #[serde(default)]
    pub ssl_enabled: bool,
    #[serde(default)]
    pub template: Option<String>, // "http", "laravel", "wordpress", "static"
    #[serde(default = "default_web_server")]
    pub web_server: String, // "nginx" or "apache"
}

fn default_web_server() -> String {
    "nginx".to_string()
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SiteWithStatus {
    pub domain: String,
    pub path: String,
    pub port: u16,
    pub php_version: Option<String>,
    pub php_port: Option<u16>,
    pub ssl_enabled: bool,
    pub template: Option<String>,
    pub web_server: String,
    pub created_at: Option<String>,
    pub config_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

pub struct SiteManager;

impl SiteManager {
    /// Get PHP port from registry or calculate based on version
    fn get_php_port(app: &AppHandle, version: &str) -> u16 {
        // Try to get from registry first
        if let Ok(registry) = PhpRegistry::load(app) {
            return registry.get_or_calculate_port(version);
        }
        // Fallback to calculation
        PhpRegistry::calculate_port(version)
    }

    /// Validate site input before creation/update
    fn validate_site_input(site: &Site) -> Result<(), String> {
        // 1. Validate domain
        validate_domain(&site.domain)
            .map_err(|e| format!("Invalid domain: {}", e))?;

        // 2. Validate port
        validate_port(site.port)
            .map_err(|e| format!("Invalid port: {}", e))?;

        // 3. Validate PHP port if specified
        if let Some(php_port) = site.php_port {
            validate_port(php_port)
                .map_err(|e| format!("Invalid PHP port: {}", e))?;
        }

        // 4. Validate path (allow any valid path for site root)
        validate_site_path(&site.path, None)
            .map_err(|e| format!("Invalid path: {}", e))?;

        // 5. Validate PHP version format if specified
        if let Some(ref version) = site.php_version {
            crate::services::validation::validate_php_version(version)
                .map_err(|e| format!("Invalid PHP version: {}", e))?;
        }

        Ok(())
    }

    /// Create a new site with full transaction support
    pub fn create_site(app: &AppHandle, site: Site) -> Result<SiteWithStatus, String> {
        // SECURITY: Validate all input first
        Self::validate_site_input(&site)?;

        // Determine which web server to use
        let use_apache = site.web_server.to_lowercase() == "apache";

        // Ensure config directory exists for the chosen web server
        let sites_dir = if use_apache {
            ApacheManager::ensure_main_config(app)?;
            ApacheManager::get_vhosts_dir(app)?
        } else {
            NginxManager::ensure_main_config(app)?;
            NginxManager::get_sites_dir(app)?
        };

        // Load store
        let mut store = SiteStore::load(app)?;

        // Check if site already exists
        if store.get_site(&site.domain).is_some() {
            return Err(format!("Site '{}' already exists", site.domain));
        }

        // Determine PHP port from registry or calculate from version
        let php_port = if let Some(ref version) = site.php_version {
            // Use provided port or get from registry
            Some(site.php_port.unwrap_or_else(|| Self::get_php_port(app, version)))
        } else {
            None
        };

        // Detect or use specified template
        let template = site
            .template
            .clone()
            .map(|t| match t.as_str() {
                "laravel" => SiteTemplate::Laravel,
                "wordpress" => SiteTemplate::WordPress,
                "litecart" => SiteTemplate::LiteCart,
                "static" => SiteTemplate::Static,
                "https" => SiteTemplate::Https,
                _ => SiteTemplate::Http,
            })
            .unwrap_or_else(|| {
                if site.php_version.is_some() {
                    SiteTemplate::detect_from_path(&site.path)
                } else {
                    SiteTemplate::Static
                }
            });

        // Build template variables with sanitization for nginx config
        let mut vars: HashMap<&str, String> = HashMap::new();
        vars.insert("domain", sanitize_for_nginx(&site.domain));
        vars.insert("port", site.port.to_string());
        vars.insert("path", sanitize_for_nginx(&site.path.replace('\\', "/")));
        // Use calculated php_port or get from registry, fallback to 9004 (PHP 8.4 default)
        let final_php_port = php_port.unwrap_or_else(|| {
            site.php_version.as_ref()
                .map(|v| Self::get_php_port(app, v))
                .unwrap_or(9004)
        });
        vars.insert("php_port", final_php_port.to_string());

        // SSL certificate paths
        let mut ssl_cert_path: Option<String> = None;
        let mut ssl_key_path: Option<String> = None;

        if site.ssl_enabled {
            // Get bin path for SSL operations
            let bin_path = app
                .path()
                .app_local_data_dir()
                .map_err(|e| e.to_string())?
                .join("bin");

            // Check if mkcert is installed
            if !SSLManager::is_mkcert_installed(&bin_path) {
                return Err("SSL is enabled but mkcert is not installed. Please install mkcert first from the SSL settings.".to_string());
            }

            // Generate SSL certificate if not exists
            let cert = if let Some(existing) = SSLManager::get_cert(&bin_path, &site.domain) {
                existing
            } else {
                SSLManager::generate_cert(&bin_path, &site.domain)?
            };

            ssl_cert_path = Some(cert.cert_path.clone());
            ssl_key_path = Some(cert.key_path.clone());

            vars.insert("ssl_port", "443".to_string());
            // Use absolute paths for SSL certificates
            vars.insert("ssl_cert", cert.cert_path.replace('\\', "/"));
            vars.insert("ssl_key", cert.key_path.replace('\\', "/"));
        }

        // Render config based on web server and SSL status
        let template_str = if use_apache {
            template.get_apache_template()
        } else if site.ssl_enabled {
            // Use SSL version of template for nginx
            match template {
                SiteTemplate::LiteCart => TEMPLATE_LITECART_SSL,
                _ => crate::services::templates::TEMPLATE_HTTPS,
            }
        } else {
            template.get_nginx_template()
        };
        let config_content = TemplateEngine::render(template_str, &vars);

        // Write config file
        let conf_path = sites_dir.join(format!("{}.conf", site.domain));
        fs::write(&conf_path, &config_content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Test config before proceeding
        let test_result = if use_apache {
            ApacheManager::test_config(app)
        } else {
            NginxManager::test_config(app)
        };

        match test_result {
            Ok(_) => {}
            Err(e) => {
                // Rollback: delete config file
                let _ = fs::remove_file(&conf_path);
                let server_name = if use_apache { "Apache" } else { "nginx" };
                return Err(format!("Invalid {} config: {}", server_name, e));
            }
        }

        // Add to hosts file (optional - warn but continue if fails)
        let hosts_warning = match HostsManager::add_domain(&site.domain) {
            Ok(_) => None,
            Err(e) => {
                log::warn!("Could not update hosts file (run as Administrator): {}", e);
                Some(format!(
                    "Note: Could not add {} to hosts file. Run as Administrator or add manually: 127.0.0.1 {}",
                    site.domain, site.domain
                ))
            }
        };

        // Create site metadata
        let now = chrono::Utc::now().to_rfc3339();
        let metadata = SiteMetadata {
            domain: site.domain.clone(),
            path: site.path.clone(),
            port: site.port,
            php_version: site.php_version.clone(),
            php_port,
            ssl_enabled: site.ssl_enabled,
            ssl_cert_path: ssl_cert_path,
            ssl_key_path: ssl_key_path,
            template: site.template.clone(),
            web_server: site.web_server.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        // Save to store
        store.add_site(metadata);
        store.save(app)?;

        // Try to reload the appropriate web server if running
        if use_apache {
            let _ = ApacheManager::reload(app);
        } else {
            let _ = NginxManager::reload(app);
        }

        Ok(SiteWithStatus {
            domain: site.domain,
            path: site.path,
            port: site.port,
            php_version: site.php_version,
            php_port,
            ssl_enabled: site.ssl_enabled,
            template: site.template,
            web_server: site.web_server,
            created_at: Some(now),
            config_valid: true,
            warning: hosts_warning,
        })
    }

    /// Get all sites from store
    pub fn get_sites(app: &AppHandle) -> Result<Vec<SiteWithStatus>, String> {
        let mut store = SiteStore::load(app)?;

        // Migration: if store is empty, try to import from nginx configs
        if store.sites.is_empty() {
            let migrated = store.migrate_from_nginx_configs(app)?;
            if migrated > 0 {
                store.save(app)?;
                log::info!("Migrated {} sites from nginx configs", migrated);
            }
        }

        let sites: Vec<SiteWithStatus> = store
            .sites
            .iter()
            .map(|s| {
                // Check if config file exists based on web server type
                let use_apache = s.web_server.to_lowercase() == "apache";
                let config_valid = if use_apache {
                    ApacheManager::get_vhosts_dir(app)
                        .map(|dir| dir.join(format!("{}.conf", s.domain)).exists())
                        .unwrap_or(false)
                } else {
                    NginxManager::get_sites_dir(app)
                        .map(|dir| dir.join(format!("{}.conf", s.domain)).exists())
                        .unwrap_or(false)
                };

                SiteWithStatus {
                    domain: s.domain.clone(),
                    path: s.path.clone(),
                    port: s.port,
                    php_version: s.php_version.clone(),
                    php_port: s.php_port,
                    ssl_enabled: s.ssl_enabled,
                    template: s.template.clone(),
                    web_server: s.web_server.clone(),
                    created_at: Some(s.created_at.clone()),
                    config_valid,
                    warning: None,
                }
            })
            .collect();

        Ok(sites)
    }

    /// Get a single site
    pub fn get_site(app: &AppHandle, domain: &str) -> Result<Option<SiteWithStatus>, String> {
        let store = SiteStore::load(app)?;

        Ok(store.get_site(domain).map(|s| {
            let use_apache = s.web_server.to_lowercase() == "apache";
            let config_valid = if use_apache {
                ApacheManager::get_vhosts_dir(app)
                    .map(|dir| dir.join(format!("{}.conf", s.domain)).exists())
                    .unwrap_or(false)
            } else {
                NginxManager::get_sites_dir(app)
                    .map(|dir| dir.join(format!("{}.conf", s.domain)).exists())
                    .unwrap_or(false)
            };

            SiteWithStatus {
                domain: s.domain.clone(),
                path: s.path.clone(),
                port: s.port,
                php_version: s.php_version.clone(),
                php_port: s.php_port,
                ssl_enabled: s.ssl_enabled,
                template: s.template.clone(),
                web_server: s.web_server.clone(),
                created_at: Some(s.created_at.clone()),
                config_valid,
                warning: None,
            }
        }))
    }

    /// Update an existing site
    pub fn update_site(app: &AppHandle, domain: &str, updates: Site) -> Result<SiteWithStatus, String> {
        // Validate existing domain
        validate_domain(domain).map_err(|e| format!("Invalid domain: {}", e))?;

        // Validate update data
        Self::validate_site_input(&updates)?;

        let mut store = SiteStore::load(app)?;

        let existing = store
            .get_site(domain)
            .ok_or_else(|| format!("Site '{}' not found", domain))?
            .clone();

        // Determine old web server type
        let old_use_apache = existing.web_server.to_lowercase() == "apache";

        // Delete old config from appropriate directory
        if old_use_apache {
            if let Ok(vhosts_dir) = ApacheManager::get_vhosts_dir(app) {
                let old_conf = vhosts_dir.join(format!("{}.conf", domain));
                if old_conf.exists() {
                    fs::remove_file(&old_conf).map_err(|e| format!("Failed to remove old config: {}", e))?;
                }
            }
        } else {
            if let Ok(sites_dir) = NginxManager::get_sites_dir(app) {
                let old_conf = sites_dir.join(format!("{}.conf", domain));
                if old_conf.exists() {
                    fs::remove_file(&old_conf).map_err(|e| format!("Failed to remove old config: {}", e))?;
                }
            }
        }

        // If domain changed, update hosts (ignore errors)
        if domain != updates.domain {
            let _ = HostsManager::remove_domain(domain);
        }

        // Remove from store
        store.remove_site(domain);
        store.save(app)?;

        // Create new site with updates
        let new_site = Site {
            domain: updates.domain,
            path: updates.path,
            port: updates.port,
            php_version: updates.php_version,
            php_port: updates.php_port.or(existing.php_port),
            ssl_enabled: updates.ssl_enabled,
            template: updates.template,
            web_server: updates.web_server,
        };

        Self::create_site(app, new_site)
    }

    /// Delete a site
    pub fn delete_site(app: &AppHandle, domain: &str) -> Result<(), String> {
        // Load and update store
        let mut store = SiteStore::load(app)?;
        let site = store.remove_site(domain);
        store.save(app)?;

        // Determine web server from site metadata
        let use_apache = site.as_ref()
            .map(|s| s.web_server.to_lowercase() == "apache")
            .unwrap_or(false);

        // Delete config from appropriate directory
        if use_apache {
            if let Ok(vhosts_dir) = ApacheManager::get_vhosts_dir(app) {
                let conf_path = vhosts_dir.join(format!("{}.conf", domain));
                if conf_path.exists() {
                    fs::remove_file(&conf_path).map_err(|e| format!("Failed to delete config: {}", e))?;
                }
            }
        } else {
            if let Ok(sites_dir) = NginxManager::get_sites_dir(app) {
                let conf_path = sites_dir.join(format!("{}.conf", domain));
                if conf_path.exists() {
                    fs::remove_file(&conf_path).map_err(|e| format!("Failed to delete config: {}", e))?;
                }
            }
        }

        // Remove from hosts
        let _ = HostsManager::remove_domain(domain);

        // Delete SSL certificates if they exist
        if let Some(ref site) = site {
            if site.ssl_enabled {
                if let Ok(config_dir) = NginxManager::get_config_dir(app) {
                    let ssl_dir = config_dir.join("ssl").join(domain);
                    if ssl_dir.exists() {
                        let _ = fs::remove_dir_all(&ssl_dir);
                    }
                }
            }
        }

        // Try to reload the appropriate web server
        if use_apache {
            let _ = ApacheManager::reload(app);
        } else {
            let _ = NginxManager::reload(app);
        }

        Ok(())
    }

    /// Regenerate config for a site
    pub fn regenerate_config(app: &AppHandle, domain: &str) -> Result<(), String> {
        let store = SiteStore::load(app)?;
        let site = store
            .get_site(domain)
            .ok_or_else(|| format!("Site '{}' not found", domain))?;

        let site_data = Site {
            domain: site.domain.clone(),
            path: site.path.clone(),
            port: site.port,
            php_version: site.php_version.clone(),
            php_port: site.php_port,
            ssl_enabled: site.ssl_enabled,
            template: site.template.clone(),
            web_server: site.web_server.clone(),
        };

        // Delete old config from appropriate directory
        let use_apache = site.web_server.to_lowercase() == "apache";
        if use_apache {
            if let Ok(vhosts_dir) = ApacheManager::get_vhosts_dir(app) {
                let conf_path = vhosts_dir.join(format!("{}.conf", domain));
                if conf_path.exists() {
                    fs::remove_file(&conf_path).ok();
                }
            }
        } else {
            if let Ok(sites_dir) = NginxManager::get_sites_dir(app) {
                let conf_path = sites_dir.join(format!("{}.conf", domain));
                if conf_path.exists() {
                    fs::remove_file(&conf_path).ok();
                }
            }
        }

        // Recreate
        let mut store = SiteStore::load(app)?;
        store.remove_site(domain);
        store.save(app)?;

        Self::create_site(app, site_data)?;

        Ok(())
    }
}
