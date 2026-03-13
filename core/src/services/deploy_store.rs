use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

// ─── New Data Model ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConnection {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    pub protocol: Protocol,
}

impl Default for ServerConnection {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: 22,
            username: String::new(),
            auth: AuthMethod::Password,
            protocol: Protocol::SSH,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum Protocol {
    SSH, // covers both SSH commands and SFTP file transfer
    FTP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Password,
    KeyFile(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployTarget {
    pub connection: String, // references ServerConnection.name
    pub remote_path: String,
}

// ─── Store ────────────────────────────────────────────────────────

pub struct DeployStore;

impl DeployStore {
    // ── Paths ──

    fn connections_path(app: &AppHandle) -> Result<PathBuf, String> {
        let config_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("config");
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        Ok(config_dir.join("deploy-connections.json"))
    }

    fn targets_path(app: &AppHandle) -> Result<PathBuf, String> {
        let config_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("config");
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        Ok(config_dir.join("deploy-targets.json"))
    }

    fn keyring_key(conn_name: &str) -> String {
        format!("orbit:deploy:{conn_name}")
    }

    // ── Migration ──

    /// Migrates old per-domain connection format to new global connections + targets format.
    /// Old format: deploy-connections.json = HashMap<domain, Vec<OldConnection>>
    /// New format: deploy-connections.json = Vec<ServerConnection>, deploy-targets.json = HashMap<domain, Vec<DeployTarget>>
    pub fn migrate_if_needed(app: &AppHandle) {
        let Ok(conn_path) = Self::connections_path(app) else {
            return;
        };
        if !conn_path.exists() {
            return;
        }

        // Try to parse as new format first (Vec<ServerConnection>)
        let Ok(data) = fs::read_to_string(&conn_path) else {
            return;
        };
        if serde_json::from_str::<Vec<ServerConnection>>(&data).is_ok() {
            return; // Already migrated
        }

        // Try to parse as old format (HashMap<domain, Vec<OldConnection>>)
        let Ok(old_data) = serde_json::from_str::<HashMap<String, Vec<OldConnection>>>(&data)
        else {
            return; // Unknown format, skip
        };

        // Extract unique global connections and build targets
        let mut global_connections: Vec<ServerConnection> = Vec::new();
        let mut targets: HashMap<String, Vec<DeployTarget>> = HashMap::new();
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (domain, conns) in &old_data {
            for old_conn in conns {
                // Migrate old keyring entry to new key format
                let old_key = format!("orbit:deploy:{domain}:{}", old_conn.name);
                let new_key = Self::keyring_key(&old_conn.name);

                if let Ok(old_entry) = keyring::Entry::new("orbit-deploy", &old_key) {
                    if let Ok(pwd) = old_entry.get_password() {
                        if let Ok(new_entry) = keyring::Entry::new("orbit-deploy", &new_key) {
                            new_entry.set_password(&pwd).ok();
                        }
                        // Don't delete old entry yet — keep as backup
                    }
                }

                // Add global connection (deduplicate by name)
                if seen_names.insert(old_conn.name.clone()) {
                    let protocol = match &old_conn.protocol {
                        OldProtocol::SSH | OldProtocol::SFTP => Protocol::SSH,
                        OldProtocol::FTP => Protocol::FTP,
                    };
                    global_connections.push(ServerConnection {
                        name: old_conn.name.clone(),
                        host: old_conn.host.clone(),
                        port: old_conn.port,
                        username: old_conn.username.clone(),
                        auth: old_conn.auth.clone(),
                        protocol,
                    });
                }

                // Create target for this domain
                targets
                    .entry(domain.clone())
                    .or_default()
                    .push(DeployTarget {
                        connection: old_conn.name.clone(),
                        remote_path: old_conn.remote_path.clone(),
                    });
            }
        }

        // Write new files
        if let Ok(json) = serde_json::to_string_pretty(&global_connections) {
            fs::write(&conn_path, json).ok();
        }
        if let Ok(targets_path) = Self::targets_path(app) {
            if let Ok(json) = serde_json::to_string_pretty(&targets) {
                fs::write(&targets_path, json).ok();
            }
        }

        // Backup old file (rename would fail since we already wrote new)
        // Old data is preserved in memory and targets file
    }

    // ── Global Connections ──

    pub fn list_connections(app: &AppHandle) -> Result<Vec<ServerConnection>, String> {
        Self::migrate_if_needed(app);
        let path = Self::connections_path(app)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn get_connection(app: &AppHandle, name: &str) -> Result<Option<ServerConnection>, String> {
        let connections = Self::list_connections(app)?;
        Ok(connections.into_iter().find(|c| c.name == name))
    }

    pub fn add_connection(
        app: &AppHandle,
        conn: ServerConnection,
        password: Option<String>,
    ) -> Result<(), String> {
        // Store password in OS keyring
        if let (AuthMethod::Password, Some(pwd)) = (&conn.auth, &password) {
            let key = Self::keyring_key(&conn.name);
            let entry = keyring::Entry::new("orbit-deploy", &key)
                .map_err(|e| format!("Keyring error: {e}"))?;
            entry
                .set_password(pwd)
                .map_err(|e| format!("Failed to store password: {e}"))?;
        }

        let mut connections = Self::list_connections(app)?;
        // Replace if same name exists
        connections.retain(|c| c.name != conn.name);
        connections.push(conn);

        let path = Self::connections_path(app)?;
        let json = serde_json::to_string_pretty(&connections).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }

    pub fn remove_connection(app: &AppHandle, name: &str) -> Result<(), String> {
        // Remove from keyring (best effort)
        let key = Self::keyring_key(name);
        if let Ok(entry) = keyring::Entry::new("orbit-deploy", &key) {
            entry.delete_credential().ok();
        }

        let mut connections = Self::list_connections(app)?;
        connections.retain(|c| c.name != name);

        let path = Self::connections_path(app)?;
        let json = serde_json::to_string_pretty(&connections).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }

    pub fn get_password(conn_name: &str) -> Result<String, String> {
        let key = Self::keyring_key(conn_name);
        let entry = keyring::Entry::new("orbit-deploy", &key)
            .map_err(|e| format!("Keyring error: {e}"))?;
        entry
            .get_password()
            .map_err(|e| format!("Failed to retrieve password: {e}"))
    }

    // ── Site Targets ──

    fn load_all_targets(app: &AppHandle) -> Result<HashMap<String, Vec<DeployTarget>>, String> {
        let path = Self::targets_path(app)?;
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    fn save_all_targets(
        app: &AppHandle,
        targets: &HashMap<String, Vec<DeployTarget>>,
    ) -> Result<(), String> {
        let path = Self::targets_path(app)?;
        let json = serde_json::to_string_pretty(targets).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }

    pub fn list_targets(app: &AppHandle, domain: &str) -> Result<Vec<DeployTarget>, String> {
        let all = Self::load_all_targets(app)?;
        Ok(all.get(domain).cloned().unwrap_or_default())
    }

    pub fn assign_target(
        app: &AppHandle,
        domain: &str,
        target: DeployTarget,
    ) -> Result<(), String> {
        // Verify connection exists
        let conn = Self::get_connection(app, &target.connection)?;
        if conn.is_none() {
            return Err(format!("Connection not found: {}", target.connection));
        }

        let mut all = Self::load_all_targets(app)?;
        let targets = all.entry(domain.to_string()).or_default();
        // Replace if same connection already assigned
        targets.retain(|t| t.connection != target.connection);
        targets.push(target);
        Self::save_all_targets(app, &all)
    }

    pub fn unassign_target(
        app: &AppHandle,
        domain: &str,
        conn_name: &str,
    ) -> Result<(), String> {
        let mut all = Self::load_all_targets(app)?;
        if let Some(targets) = all.get_mut(domain) {
            targets.retain(|t| t.connection != conn_name);
        }
        Self::save_all_targets(app, &all)
    }
}

// ─── Old Format (for migration) ──────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct OldConnection {
    name: String,
    protocol: OldProtocol,
    host: String,
    port: u16,
    username: String,
    auth: AuthMethod,
    remote_path: String,
}

impl Default for OldConnection {
    fn default() -> Self {
        Self {
            name: String::new(),
            protocol: OldProtocol::SSH,
            host: String::new(),
            port: 22,
            username: String::new(),
            auth: AuthMethod::Password,
            remote_path: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
enum OldProtocol {
    #[default]
    SSH,
    SFTP,
    FTP,
}
