use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeployConnection {
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    pub remote_path: String,
}

impl Default for DeployConnection {
    fn default() -> Self {
        Self {
            name: String::new(),
            protocol: Protocol::SSH,
            host: String::new(),
            port: 22,
            username: String::new(),
            auth: AuthMethod::Password,
            remote_path: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum Protocol {
    SSH,
    SFTP,
    FTP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Password,
    KeyFile(String),
}

pub struct DeployStore;

impl DeployStore {
    fn get_store_path(app: &AppHandle) -> Result<PathBuf, String> {
        let config_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("config");
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        Ok(config_dir.join("deploy-connections.json"))
    }

    fn keyring_key(domain: &str, conn_name: &str) -> String {
        format!("orbit:deploy:{domain}:{conn_name}")
    }

    pub fn load_all(
        app: &AppHandle,
    ) -> Result<HashMap<String, Vec<DeployConnection>>, String> {
        let path = Self::get_store_path(app)?;
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn load_for_site(
        app: &AppHandle,
        domain: &str,
    ) -> Result<Vec<DeployConnection>, String> {
        let all = Self::load_all(app)?;
        Ok(all.get(domain).cloned().unwrap_or_default())
    }

    pub fn save_all(
        app: &AppHandle,
        data: &HashMap<String, Vec<DeployConnection>>,
    ) -> Result<(), String> {
        let path = Self::get_store_path(app)?;
        let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }

    pub fn add_connection(
        app: &AppHandle,
        domain: &str,
        connection: DeployConnection,
        password: Option<String>,
    ) -> Result<(), String> {
        // Store password in OS keyring
        if let (AuthMethod::Password, Some(pwd)) = (&connection.auth, &password) {
            let key = Self::keyring_key(domain, &connection.name);
            let entry = keyring::Entry::new("orbit-deploy", &key)
                .map_err(|e| format!("Keyring error: {e}"))?;
            entry
                .set_password(pwd)
                .map_err(|e| format!("Failed to store password: {e}"))?;
        }

        // Save metadata to JSON
        let mut all = Self::load_all(app)?;
        let connections = all.entry(domain.to_string()).or_default();
        // Replace if same name exists
        connections.retain(|c| c.name != connection.name);
        connections.push(connection);
        Self::save_all(app, &all)
    }

    pub fn remove_connection(
        app: &AppHandle,
        domain: &str,
        conn_name: &str,
    ) -> Result<(), String> {
        // Remove from keyring (best effort)
        let key = Self::keyring_key(domain, conn_name);
        if let Ok(entry) = keyring::Entry::new("orbit-deploy", &key) {
            entry.delete_credential().ok();
        }

        // Remove from JSON
        let mut all = Self::load_all(app)?;
        if let Some(connections) = all.get_mut(domain) {
            connections.retain(|c| c.name != conn_name);
        }
        Self::save_all(app, &all)
    }

    pub fn get_password(domain: &str, conn_name: &str) -> Result<String, String> {
        let key = Self::keyring_key(domain, conn_name);
        let entry = keyring::Entry::new("orbit-deploy", &key)
            .map_err(|e| format!("Keyring error: {e}"))?;
        entry
            .get_password()
            .map_err(|e| format!("Failed to retrieve password: {e}"))
    }
}
