use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteMetadata {
    pub domain: String,
    pub path: String,
    pub port: u16,
    pub php_version: Option<String>,
    pub php_port: Option<u16>,
    pub ssl_enabled: bool,
    pub ssl_cert_path: Option<String>,
    pub ssl_key_path: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default = "default_web_server")]
    pub web_server: String,
    #[serde(default)]
    pub dev_port: Option<u16>,
    #[serde(default = "default_timestamp")]
    pub created_at: String,
    #[serde(default = "default_timestamp")]
    pub updated_at: String,
}

fn default_web_server() -> String {
    "nginx".to_string()
}

fn default_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SiteStore {
    pub version: String,
    pub sites: Vec<SiteMetadata>,
}

impl SiteStore {
    fn get_store_path(app: &AppHandle) -> Result<PathBuf, String> {
        let config_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("config");

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        Ok(config_dir.join("sites.json"))
    }

    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let path = Self::get_store_path(app)?;

        if !path.exists() {
            return Ok(SiteStore {
                version: "1.0".to_string(),
                sites: vec![],
            });
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read sites store: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse sites store: {}", e))
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let path = Self::get_store_path(app)?;

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize sites: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Failed to write sites store: {}", e))
    }

    pub fn add_site(&mut self, site: SiteMetadata) {
        // Remove existing site with same domain if any
        self.sites.retain(|s| s.domain != site.domain);
        self.sites.push(site);
    }

    pub fn remove_site(&mut self, domain: &str) -> Option<SiteMetadata> {
        if let Some(pos) = self.sites.iter().position(|s| s.domain == domain) {
            Some(self.sites.remove(pos))
        } else {
            None
        }
    }

    pub fn get_site(&self, domain: &str) -> Option<&SiteMetadata> {
        self.sites.iter().find(|s| s.domain == domain)
    }

    #[allow(dead_code)]
    pub fn get_site_mut(&mut self, domain: &str) -> Option<&mut SiteMetadata> {
        self.sites.iter_mut().find(|s| s.domain == domain)
    }

    #[allow(dead_code)]
    pub fn update_site(&mut self, domain: &str, updates: SiteMetadata) -> bool {
        if let Some(site) = self.get_site_mut(domain) {
            *site = updates;
            true
        } else {
            false
        }
    }

    /// Migrate existing nginx configs to store (one-time migration)
    pub fn migrate_from_nginx_configs(&mut self, app: &AppHandle) -> Result<usize, String> {
        let sites_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("nginx")
            .join("conf")
            .join("sites-enabled");

        if !sites_dir.exists() {
            return Ok(0);
        }

        let mut migrated = 0;

        if let Ok(entries) = fs::read_dir(&sites_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "conf").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(site) = Self::parse_nginx_config(&content) {
                            // Only add if not already in store
                            if self.get_site(&site.domain).is_none() {
                                self.add_site(site);
                                migrated += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(migrated)
    }

    fn parse_nginx_config(content: &str) -> Option<SiteMetadata> {
        let domain = content
            .lines()
            .find(|l| l.trim().starts_with("server_name"))
            .and_then(|l| l.split_whitespace().nth(1))
            .map(|s| s.trim_end_matches(';').to_string())?;

        let port = content
            .lines()
            .find(|l| l.trim().starts_with("listen"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.trim_end_matches(';').parse::<u16>().ok())
            .unwrap_or(80);

        let path = content
            .lines()
            .find(|l| l.trim().starts_with("root"))
            .and_then(|l| {
                let rest = l.trim().strip_prefix("root")?;
                let rest = rest.trim().trim_end_matches(';');
                Some(rest.trim_matches('"').to_string())
            })
            .unwrap_or_default();

        // Try to extract PHP port from fastcgi_pass
        let php_port = content
            .lines()
            .find(|l| l.contains("fastcgi_pass"))
            .and_then(|l| {
                l.split(':')
                    .last()
                    .and_then(|s| s.trim_end_matches(';').trim().parse::<u16>().ok())
            });

        let ssl_enabled = content.contains("ssl_certificate");

        let now = chrono::Utc::now().to_rfc3339();

        Some(SiteMetadata {
            domain,
            path,
            port,
            php_version: None, // Can't determine from config
            php_port,
            ssl_enabled,
            ssl_cert_path: None,
            ssl_key_path: None,
            template: None,
            web_server: "nginx".to_string(), // Migrated from nginx config
            dev_port: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

/// Get the next available PHP port starting from base
#[allow(dead_code)]
pub fn get_next_php_port(store: &SiteStore, base_port: u16) -> u16 {
    let used_ports: Vec<u16> = store
        .sites
        .iter()
        .filter_map(|s| s.php_port)
        .collect();

    let mut port = base_port;
    while used_ports.contains(&port) {
        port += 1;
    }
    port
}
