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
    #[serde(default)]
    pub dev_command: Option<String>,
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
            dev_command: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_site(domain: &str) -> SiteMetadata {
        SiteMetadata {
            domain: domain.to_string(),
            path: "/var/www/test".to_string(),
            port: 80,
            php_version: Some("8.4".to_string()),
            php_port: Some(9004),
            ssl_enabled: false,
            ssl_cert_path: None,
            ssl_key_path: None,
            template: Some("laravel".to_string()),
            web_server: "nginx".to_string(),
            dev_port: None,
            dev_command: None,
            created_at: default_timestamp(),
            updated_at: default_timestamp(),
        }
    }

    #[test]
    fn test_site_store_crud() {
        let mut store = SiteStore {
            version: "1.0".to_string(),
            sites: vec![],
        };

        // Add
        let site1 = create_test_site("test1.local");
        store.add_site(site1.clone());
        assert_eq!(store.sites.len(), 1);

        // Add duplicate domain (should replace)
        let mut site1_updated = create_test_site("test1.local");
        site1_updated.port = 8080;
        store.add_site(site1_updated);
        assert_eq!(store.sites.len(), 1);
        assert_eq!(store.sites[0].port, 8080);

        // Add second site
        let site2 = create_test_site("test2.local");
        store.add_site(site2);
        assert_eq!(store.sites.len(), 2);

        // Get
        assert!(store.get_site("test1.local").is_some());
        assert!(store.get_site("test3.local").is_none());

        // Get mut
        if let Some(site) = store.get_site_mut("test1.local") {
            site.port = 9090;
        }
        assert_eq!(store.get_site("test1.local").unwrap().port, 9090);

        // Update
        let mut updates = create_test_site("test2.local");
        updates.port = 8888;
        assert!(store.update_site("test2.local", updates));
        assert_eq!(store.get_site("test2.local").unwrap().port, 8888);

        // Update non-existent
        let updates = create_test_site("nonexistent.local");
        assert!(!store.update_site("nonexistent.local", updates));

        // Remove
        let removed = store.remove_site("test1.local");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().domain, "test1.local");
        assert_eq!(store.sites.len(), 1);

        // Remove non-existent
        assert!(store.remove_site("test1.local").is_none());
    }

    #[test]
    fn test_parse_nginx_config() {
        let config = r#"
            server {
                listen 80;
                server_name example.test;
                root "/var/www/example";
                location ~ \.php$ {
                    fastcgi_pass 127.0.0.1:9004;
                }
            }
        "#;

        let site = SiteStore::parse_nginx_config(config).unwrap();
        assert_eq!(site.domain, "example.test");
        assert_eq!(site.port, 80);
        assert_eq!(site.path, "/var/www/example");
        assert_eq!(site.php_port, Some(9004));
        assert!(!site.ssl_enabled);
        assert_eq!(site.web_server, "nginx");
    }

    #[test]
    fn test_get_next_php_port() {
        let mut store = SiteStore {
            version: "1".to_string(),
            sites: vec![],
        };
        
        let mut site1 = create_test_site("test1");
        site1.php_port = Some(9000);
        store.add_site(site1);

        let mut site2 = create_test_site("test2");
        site2.php_port = Some(9001);
        store.add_site(site2);

        assert_eq!(get_next_php_port(&store, 9000), 9002);
        assert_eq!(get_next_php_port(&store, 9005), 9005);
    }

    #[test]
    fn test_site_metadata_serde() {
        let site = create_test_site("serde.test");
        let json = serde_json::to_string(&site).unwrap();
        
        let deserialized: SiteMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.domain, "serde.test");
        assert_eq!(deserialized.port, 80);
        assert_eq!(deserialized.web_server, "nginx"); // Default was used
    }

    #[test]
    fn test_site_store_serde() {
        let mut store = SiteStore {
            version: "1.0".to_string(),
            sites: vec![],
        };
        store.add_site(create_test_site("store1.test"));
        store.add_site(create_test_site("store2.test"));

        let json = serde_json::to_string(&store).unwrap();
        let deserialized: SiteStore = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, "1.0");
        assert_eq!(deserialized.sites.len(), 2);
        assert_eq!(deserialized.sites[0].domain, "store1.test");
    }

    #[test]
    fn test_site_metadata_optional_fields() {
        // Test missing optional fields
        let json = r#"{
            "domain": "minimal.test",
            "path": "/var/www/minimal",
            "port": 80,
            "ssl_enabled": false
        }"#;

        let site: SiteMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(site.domain, "minimal.test");
        assert_eq!(site.php_version, None);
        assert_eq!(site.template, None);
        assert_eq!(site.web_server, "nginx"); // default
    }
}
