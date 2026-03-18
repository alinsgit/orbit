use crate::services::deploy_store::{AuthMethod, DeployStore, Protocol, ServerConnection};
use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::fs;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use suppaftp::FtpStream;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHash {
    pub path: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployManifest {
    pub timestamp: String,
    pub domain: String,
    pub connection: String,
    pub files: Vec<FileHash>,
    pub status: DeployStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeployStatus {
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployProgress {
    pub domain: String,
    pub connection: String,
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub file: Option<String>,
}

pub struct DeployService;

impl DeployService {
    // ─── Connection Testing ──────────────────────────────────────────

    pub fn test_connection(conn: &ServerConnection) -> Result<String, String> {
        match conn.protocol {
            Protocol::SSH => Self::test_ssh(conn),
            Protocol::FTP => Self::test_ftp(conn),
        }
    }

    fn test_ssh(conn: &ServerConnection) -> Result<String, String> {
        let session = Self::create_ssh_session(conn)?;
        if session.authenticated() {
            Ok(format!("SSH connection successful to {}", conn.host))
        } else {
            Err("Authentication failed".to_string())
        }
    }

    fn test_ftp(conn: &ServerConnection) -> Result<String, String> {
        let addr = format!("{}:{}", conn.host, conn.port);
        let mut ftp =
            FtpStream::connect(&addr).map_err(|e| format!("FTP connection failed: {e}"))?;

        let password = match &conn.auth {
            AuthMethod::Password => DeployStore::get_password(&conn.name)?,
            AuthMethod::KeyFile(_) => {
                return Err("FTP does not support key authentication".to_string())
            }
        };

        ftp.login(&conn.username, &password)
            .map_err(|e| format!("FTP login failed: {e}"))?;
        ftp.quit().ok();

        Ok(format!("FTP connection successful to {}", conn.host))
    }

    // ─── SSH Session Factory ─────────────────────────────────────────

    fn create_ssh_session(conn: &ServerConnection) -> Result<Session, String> {
        let addr = format!("{}:{}", conn.host, conn.port);
        let tcp =
            TcpStream::connect(&addr).map_err(|e| format!("Connection failed: {e}"))?;
        let mut session = Session::new().map_err(|e| format!("SSH error: {e}"))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| format!("Handshake failed: {e}"))?;

        match &conn.auth {
            AuthMethod::Password => {
                let password = DeployStore::get_password(&conn.name)?;
                session
                    .userauth_password(&conn.username, &password)
                    .map_err(|e| format!("Auth failed: {e}"))?;
            }
            AuthMethod::KeyFile(path) => {
                session
                    .userauth_pubkey_file(
                        &conn.username,
                        None,
                        Path::new(path),
                        None,
                    )
                    .map_err(|e| format!("Key auth failed: {e}"))?;
            }
        }

        Ok(session)
    }

    // ─── Remote Execution ────────────────────────────────────────────

    pub fn ssh_execute(conn: &ServerConnection, command: &str) -> Result<String, String> {
        let session = Self::create_ssh_session(conn)?;
        let mut channel = session
            .channel_session()
            .map_err(|e| format!("Channel error: {e}"))?;
        channel
            .exec(command)
            .map_err(|e| format!("Exec error: {e}"))?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| format!("Read error: {e}"))?;
        channel.wait_close().ok();

        Ok(output)
    }

    // ─── File Hashing ────────────────────────────────────────────────

    pub fn hash_local_files(site_path: &Path) -> Result<Vec<FileHash>, String> {
        use ignore::WalkBuilder;

        let deployignore = site_path.join(".deployignore");
        let mut builder = WalkBuilder::new(site_path);
        builder.hidden(false).git_ignore(true).git_global(false);

        if deployignore.exists() {
            builder.add_ignore(&deployignore);
        }

        let mut files = Vec::new();
        for entry in builder.build().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let relative = path
                .strip_prefix(site_path)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");

            // Mandatory exclusions
            if relative.starts_with(".git/")
                || relative == ".git"
                || relative == ".env"
                || relative.starts_with(".env.")
                || relative.starts_with("node_modules/")
                || relative == ".DS_Store"
                || relative == "Thumbs.db"
            {
                continue;
            }

            let content = fs::read(path).map_err(|e| e.to_string())?;
            let hash = blake3::hash(&content).to_hex().to_string();
            let size = content.len() as u64;
            files.push(FileHash {
                path: relative,
                hash,
                size,
            });
        }
        Ok(files)
    }

    /// Compare local and remote file hashes — returns (added, modified, deleted)
    pub fn calculate_diff(
        local: &[FileHash],
        remote: &[FileHash],
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        use std::collections::HashMap;

        let remote_map: HashMap<&str, &str> = remote
            .iter()
            .map(|f| (f.path.as_str(), f.hash.as_str()))
            .collect();
        let local_map: HashMap<&str, &str> = local
            .iter()
            .map(|f| (f.path.as_str(), f.hash.as_str()))
            .collect();

        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        for file in local {
            match remote_map.get(file.path.as_str()) {
                None => added.push(file.path.clone()),
                Some(remote_hash) => {
                    if *remote_hash != file.hash.as_str() {
                        modified.push(file.path.clone());
                    }
                }
            }
        }

        for file in remote {
            if !local_map.contains_key(file.path.as_str()) {
                deleted.push(file.path.clone());
            }
        }

        (added, modified, deleted)
    }

    // ─── Unified Sync Entry Point ────────────────────────────────────

    pub fn sync(
        app: &AppHandle,
        domain: &str,
        conn: &ServerConnection,
        remote_path: &str,
        site_path: &Path,
    ) -> Result<DeployManifest, String> {
        match conn.protocol {
            Protocol::SSH => Self::sync_sftp(app, domain, conn, remote_path, site_path),
            Protocol::FTP => Self::sync_ftp(app, domain, conn, remote_path, site_path),
        }
    }

    // ─── SFTP Sync ──────────────────────────────────────────────────

    fn sync_sftp(
        app: &AppHandle,
        domain: &str,
        conn: &ServerConnection,
        remote_path: &str,
        site_path: &Path,
    ) -> Result<DeployManifest, String> {
        Self::acquire_lock(app, domain)?;

        let conn_name = conn.name.clone();
        let result = (|| {
            let session = Self::create_ssh_session(conn)?;
            let sftp = session.sftp().map_err(|e| format!("SFTP error: {e}"))?;

            // Hash local files
            app.emit(
                "deploy-progress",
                DeployProgress {
                    domain: domain.to_string(),
                    connection: conn_name.clone(),
                    phase: "hashing".to_string(),
                    current: 0,
                    total: 0,
                    file: None,
                },
            )
            .ok();

            let local_files = Self::hash_local_files(site_path)?;

            // Load previous manifest for diff
            let manifest_path = Self::manifest_path(app, domain, &conn_name)?;
            let remote_files: Vec<FileHash> = if manifest_path.exists() {
                let data = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
                let manifest: DeployManifest =
                    serde_json::from_str(&data).map_err(|e| e.to_string())?;
                manifest.files
            } else {
                Vec::new()
            };

            let (added, modified, deleted) = Self::calculate_diff(&local_files, &remote_files);
            let upload_list: Vec<&str> = added
                .iter()
                .chain(modified.iter())
                .map(|s| s.as_str())
                .collect();
            let total = upload_list.len() + deleted.len();

            // Upload files
            for (i, relative) in upload_list.iter().enumerate() {
                let local_path = site_path.join(relative);
                let remote_full = format!(
                    "{}/{}",
                    remote_path.trim_end_matches('/'),
                    relative
                );

                if let Some(parent) = Path::new(&remote_full).parent() {
                    Self::sftp_mkdir_recursive(&sftp, &parent.to_string_lossy())?;
                }

                let content = fs::read(&local_path).map_err(|e| e.to_string())?;
                let mut remote_file = sftp
                    .create(Path::new(&remote_full))
                    .map_err(|e| format!("SFTP create error: {e}"))?;
                std::io::Write::write_all(&mut remote_file, &content)
                    .map_err(|e| format!("SFTP write error: {e}"))?;

                app.emit(
                    "deploy-progress",
                    DeployProgress {
                        domain: domain.to_string(),
                        connection: conn_name.clone(),
                        phase: "uploading".to_string(),
                        current: i + 1,
                        total,
                        file: Some(relative.to_string()),
                    },
                )
                .ok();
            }

            // Delete removed files
            for (i, relative) in deleted.iter().enumerate() {
                let remote_full = format!(
                    "{}/{}",
                    remote_path.trim_end_matches('/'),
                    relative
                );
                sftp.unlink(Path::new(&remote_full)).ok();

                app.emit(
                    "deploy-progress",
                    DeployProgress {
                        domain: domain.to_string(),
                        connection: conn_name.clone(),
                        phase: "deleting".to_string(),
                        current: upload_list.len() + i + 1,
                        total,
                        file: Some(relative.clone()),
                    },
                )
                .ok();
            }

            // Save manifest
            let timestamp = chrono::Utc::now().to_rfc3339();
            let manifest = DeployManifest {
                timestamp,
                domain: domain.to_string(),
                connection: conn_name.clone(),
                files: local_files,
                status: DeployStatus::Completed,
            };
            let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
            fs::write(&manifest_path, &json).map_err(|e| e.to_string())?;

            Self::cleanup_old_manifests(app, domain).ok();

            Ok(manifest)
        })();

        Self::release_lock(app, domain);
        result
    }

    // ─── FTP Sync ───────────────────────────────────────────────────

    fn sync_ftp(
        app: &AppHandle,
        domain: &str,
        conn: &ServerConnection,
        remote_path: &str,
        site_path: &Path,
    ) -> Result<DeployManifest, String> {
        Self::acquire_lock(app, domain)?;

        let conn_name = conn.name.clone();
        let result = (|| {
            let password = match &conn.auth {
                AuthMethod::Password => DeployStore::get_password(&conn.name)?,
                AuthMethod::KeyFile(_) => {
                    return Err("FTP does not support key authentication".to_string())
                }
            };

            let addr = format!("{}:{}", conn.host, conn.port);
            let mut ftp = FtpStream::connect(&addr)
                .map_err(|e| format!("FTP connection failed: {e}"))?;
            ftp.login(&conn.username, &password)
                .map_err(|e| format!("FTP login failed: {e}"))?;

            let local_files = Self::hash_local_files(site_path)?;

            // Load previous manifest for diff
            let manifest_path = Self::manifest_path(app, domain, &conn_name)?;
            let remote_files: Vec<FileHash> = if manifest_path.exists() {
                let data = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
                let manifest: DeployManifest =
                    serde_json::from_str(&data).map_err(|e| e.to_string())?;
                manifest.files
            } else {
                Vec::new()
            };

            let (added, modified, deleted) = Self::calculate_diff(&local_files, &remote_files);
            let upload_list: Vec<&str> = added
                .iter()
                .chain(modified.iter())
                .map(|s| s.as_str())
                .collect();
            let total = upload_list.len() + deleted.len();

            // Upload files
            for (i, relative) in upload_list.iter().enumerate() {
                let local_path = site_path.join(relative);
                let remote_full = format!(
                    "{}/{}",
                    remote_path.trim_end_matches('/'),
                    relative
                );

                if let Some(parent) = Path::new(&remote_full).parent() {
                    Self::ftp_mkdir_recursive(&mut ftp, &parent.to_string_lossy())?;
                }

                let content = fs::read(&local_path).map_err(|e| e.to_string())?;
                let mut cursor = std::io::Cursor::new(content);
                ftp.put_file(&remote_full, &mut cursor)
                    .map_err(|e| format!("FTP upload error: {e}"))?;

                app.emit(
                    "deploy-progress",
                    DeployProgress {
                        domain: domain.to_string(),
                        connection: conn_name.clone(),
                        phase: "uploading".to_string(),
                        current: i + 1,
                        total,
                        file: Some(relative.to_string()),
                    },
                )
                .ok();
            }

            // Delete removed files
            for (i, relative) in deleted.iter().enumerate() {
                let remote_full = format!(
                    "{}/{}",
                    remote_path.trim_end_matches('/'),
                    relative
                );
                ftp.rm(&remote_full).ok();

                app.emit(
                    "deploy-progress",
                    DeployProgress {
                        domain: domain.to_string(),
                        connection: conn_name.clone(),
                        phase: "deleting".to_string(),
                        current: upload_list.len() + i + 1,
                        total,
                        file: Some(relative.clone()),
                    },
                )
                .ok();
            }

            ftp.quit().ok();

            let timestamp = chrono::Utc::now().to_rfc3339();
            let manifest = DeployManifest {
                timestamp,
                domain: domain.to_string(),
                connection: conn_name.clone(),
                files: local_files,
                status: DeployStatus::Completed,
            };
            let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
            fs::write(&manifest_path, &json).map_err(|e| e.to_string())?;

            Self::cleanup_old_manifests(app, domain).ok();

            Ok(manifest)
        })();

        Self::release_lock(app, domain);
        result
    }

    // ─── Deploy Lock ─────────────────────────────────────────────────

    fn lock_path(app: &AppHandle, domain: &str) -> Result<std::path::PathBuf, String> {
        let dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("config")
            .join("deploy-locks");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir.join(format!("{domain}.lock")))
    }

    fn acquire_lock(app: &AppHandle, domain: &str) -> Result<(), String> {
        let lock = Self::lock_path(app, domain)?;
        if lock.exists() {
            return Err(format!(
                "Deploy already in progress for {domain}. Wait for it to finish."
            ));
        }
        fs::write(&lock, chrono::Utc::now().to_rfc3339()).map_err(|e| e.to_string())
    }

    fn release_lock(app: &AppHandle, domain: &str) {
        if let Ok(lock) = Self::lock_path(app, domain) {
            fs::remove_file(&lock).ok();
        }
    }

    // ─── Manifest Storage ────────────────────────────────────────────

    fn manifest_dir(app: &AppHandle, domain: &str) -> Result<std::path::PathBuf, String> {
        let dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("config")
            .join("deploy-manifests")
            .join(domain);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir)
    }

    fn manifest_path(
        app: &AppHandle,
        domain: &str,
        conn_name: &str,
    ) -> Result<std::path::PathBuf, String> {
        let dir = Self::manifest_dir(app, domain)?;
        Ok(dir.join(format!("{conn_name}.json")))
    }

    pub fn get_last_manifest(
        app: &AppHandle,
        domain: &str,
        conn_name: &str,
    ) -> Result<Option<DeployManifest>, String> {
        let path = Self::manifest_path(app, domain, conn_name)?;
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let manifest: DeployManifest =
            serde_json::from_str(&data).map_err(|e| e.to_string())?;
        Ok(Some(manifest))
    }

    fn cleanup_old_manifests(app: &AppHandle, domain: &str) -> Result<(), String> {
        let dir = Self::manifest_dir(app, domain)?;
        if !dir.exists() {
            return Ok(());
        }
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        entries.reverse();
        for entry in entries.iter().skip(10) {
            fs::remove_file(entry.path()).ok();
        }
        Ok(())
    }

    // ─── SFTP Helpers ────────────────────────────────────────────────

    fn sftp_mkdir_recursive(sftp: &ssh2::Sftp, path: &str) -> Result<(), String> {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let mut current = String::new();
        for part in parts {
            current = format!("{}/{}", current, part);
            sftp.mkdir(Path::new(&current), 0o755).ok();
        }
        Ok(())
    }

    // ─── FTP Helpers ─────────────────────────────────────────────────

    fn ftp_mkdir_recursive(ftp: &mut FtpStream, path: &str) -> Result<(), String> {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let mut current = String::new();
        for part in parts {
            current = format!("{}/{}", current, part);
            ftp.mkdir(&current).ok();
        }
        Ok(())
    }
}
