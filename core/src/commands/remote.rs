// Tauri commands for the Remote workspace: SFTP file browsing/management,
// local filesystem browsing (dual-pane explorer), and interactive SSH shells.
//
// SFTP ops run against the pooled session (services::ssh_session::SshConnPool)
// so browsing stays responsive. SSH shells get their own session per the
// channel-ownership constraints documented in ssh_session.rs.

use crate::services::deploy_store::DeployStore;
use crate::services::ssh_session::{
    self, SshConnPool, SshTerminalState,
};
use serde::{Deserialize, Serialize};
use ssh2::Sftp;
use std::fs;
use std::path::Path;
use tauri::{command, AppHandle, State};

// ─── Shared Entry Types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    /// Unix mtime in seconds (0 if unknown).
    pub modified: u64,
    /// Unix permission bits (0 if unknown / not applicable).
    pub permissions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirListing {
    /// Normalized absolute path that was listed.
    pub path: String,
    pub entries: Vec<FileEntry>,
}

// ─── Local Filesystem ────────────────────────────────────────────────

#[command]
pub fn local_list_dir(path: String) -> Result<DirListing, String> {
    let dir = Path::new(&path);
    let dir = if path.is_empty() {
        dirs_home()?
    } else {
        dir.to_path_buf()
    };
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", dir.display()));
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        entries.push(FileEntry {
            name,
            path: entry.path().to_string_lossy().to_string(),
            is_dir: meta.is_dir(),
            size: meta.len(),
            modified,
            permissions: 0,
        });
    }

    sort_entries(&mut entries);
    Ok(DirListing {
        path: dir.to_string_lossy().to_string(),
        entries,
    })
}

#[command]
pub fn local_mkdir(path: String) -> Result<(), String> {
    fs::create_dir_all(&path).map_err(|e| e.to_string())
}

#[command]
pub fn local_delete(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.is_dir() {
        fs::remove_dir_all(p).map_err(|e| e.to_string())
    } else {
        fs::remove_file(p).map_err(|e| e.to_string())
    }
}

#[command]
pub fn local_rename(from: String, to: String) -> Result<(), String> {
    fs::rename(&from, &to).map_err(|e| e.to_string())
}

/// The user's home directory — used as the default local pane root.
#[command]
pub fn local_home_dir() -> Result<String, String> {
    Ok(dirs_home()?.to_string_lossy().to_string())
}

fn dirs_home() -> Result<std::path::PathBuf, String> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "Could not resolve home directory".to_string())
}

// ─── SFTP Browsing ───────────────────────────────────────────────────

#[command]
pub fn sftp_list_dir(
    app: AppHandle,
    pool: State<'_, SshConnPool>,
    connection: String,
    path: String,
) -> Result<DirListing, String> {
    pool.with_session(&app, &connection, |sess| {
        let sftp = sess.sftp().map_err(|e| format!("SFTP error: {e}"))?;

        // Default to the connection's home directory when no path is given.
        let base = if path.is_empty() {
            sftp_home(&sftp).unwrap_or_else(|| ".".to_string())
        } else {
            path
        };
        let base = normalize_remote(&base);

        let mut entries = Vec::new();
        let readdir = sftp
            .readdir(Path::new(&base))
            .map_err(|e| format!("Failed to list '{base}': {e}"))?;

        for (entry_path, stat) in readdir {
            let name = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.is_empty() || name == "." || name == ".." {
                continue;
            }
            let full = join_remote(&base, &name);
            entries.push(FileEntry {
                name,
                path: full,
                is_dir: stat.is_dir(),
                size: stat.size.unwrap_or(0),
                modified: stat.mtime.unwrap_or(0),
                permissions: stat.perm.unwrap_or(0),
            });
        }

        sort_entries(&mut entries);
        Ok(DirListing {
            path: base,
            entries,
        })
    })
}

#[command]
pub fn sftp_mkdir(
    app: AppHandle,
    pool: State<'_, SshConnPool>,
    connection: String,
    path: String,
) -> Result<(), String> {
    pool.with_session(&app, &connection, |sess| {
        let sftp = sess.sftp().map_err(|e| format!("SFTP error: {e}"))?;
        sftp.mkdir(Path::new(&normalize_remote(&path)), 0o755)
            .map_err(|e| format!("mkdir failed: {e}"))
    })
}

#[command]
pub fn sftp_delete(
    app: AppHandle,
    pool: State<'_, SshConnPool>,
    connection: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    pool.with_session(&app, &connection, |sess| {
        let sftp = sess.sftp().map_err(|e| format!("SFTP error: {e}"))?;
        let p = normalize_remote(&path);
        if is_dir {
            sftp_remove_dir_recursive(&sftp, &p)
        } else {
            sftp.unlink(Path::new(&p))
                .map_err(|e| format!("delete failed: {e}"))
        }
    })
}

#[command]
pub fn sftp_rename(
    app: AppHandle,
    pool: State<'_, SshConnPool>,
    connection: String,
    from: String,
    to: String,
) -> Result<(), String> {
    pool.with_session(&app, &connection, |sess| {
        let sftp = sess.sftp().map_err(|e| format!("SFTP error: {e}"))?;
        sftp.rename(
            Path::new(&normalize_remote(&from)),
            Path::new(&normalize_remote(&to)),
            None,
        )
        .map_err(|e| format!("rename failed: {e}"))
    })
}

/// Download a remote file or directory (recursive) to a local destination.
#[command]
pub fn sftp_download_path(
    app: AppHandle,
    pool: State<'_, SshConnPool>,
    connection: String,
    remote_path: String,
    local_path: String,
) -> Result<String, String> {
    pool.with_session(&app, &connection, |sess| {
        let sftp = sess.sftp().map_err(|e| format!("SFTP error: {e}"))?;
        let remote = normalize_remote(&remote_path);

        let stat = sftp
            .stat(Path::new(&remote))
            .map_err(|e| format!("stat failed: {e}"))?;
        let count = if stat.is_dir() {
            sftp_download_dir(&sftp, &remote, Path::new(&local_path))?
        } else {
            sftp_download_file(&sftp, &remote, Path::new(&local_path))?;
            1
        };
        Ok(format!("Downloaded {count} file(s)"))
    })
}

/// Upload a local file or directory (recursive) to a remote destination.
#[command]
pub fn sftp_upload_path(
    app: AppHandle,
    pool: State<'_, SshConnPool>,
    connection: String,
    local_path: String,
    remote_path: String,
) -> Result<String, String> {
    pool.with_session(&app, &connection, |sess| {
        let sftp = sess.sftp().map_err(|e| format!("SFTP error: {e}"))?;
        let local = Path::new(&local_path);
        let remote = normalize_remote(&remote_path);

        let count = if local.is_dir() {
            sftp_upload_dir(&sftp, local, &remote)?
        } else {
            sftp_upload_file(&sftp, local, &remote)?;
            1
        };
        Ok(format!("Uploaded {count} file(s)"))
    })
}

#[command]
pub fn sftp_disconnect(pool: State<'_, SshConnPool>, connection: String) -> Result<(), String> {
    pool.disconnect(&connection);
    Ok(())
}

// ─── Interactive SSH Terminal ────────────────────────────────────────

#[command]
pub fn ssh_spawn_terminal(
    app: AppHandle,
    state: State<'_, SshTerminalState>,
    connection: String,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let conn = DeployStore::get_connection(&app, &connection)?
        .ok_or_else(|| format!("Connection not found: {connection}"))?;
    ssh_session::spawn_ssh_terminal(app.clone(), &state, &conn, id, cols, rows)
}

#[command]
pub fn ssh_write_terminal(
    state: State<'_, SshTerminalState>,
    id: String,
    data: String,
) -> Result<(), String> {
    ssh_session::write_ssh_terminal(&state, &id, data)
}

#[command]
pub fn ssh_resize_terminal(
    state: State<'_, SshTerminalState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    ssh_session::resize_ssh_terminal(&state, &id, cols, rows)
}

#[command]
pub fn ssh_close_terminal(
    state: State<'_, SshTerminalState>,
    id: String,
) -> Result<(), String> {
    ssh_session::close_ssh_terminal(&state, &id)
}

// ─── Helpers ─────────────────────────────────────────────────────────

/// Directories first, then case-insensitive name order.
fn sort_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Collapse backslashes and redundant slashes; keep a single leading slash.
fn normalize_remote(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    let leading = replaced.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for part in replaced.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            p => out.push(p),
        }
    }
    let joined = out.join("/");
    if leading {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

fn join_remote(base: &str, name: &str) -> String {
    normalize_remote(&format!("{}/{}", base.trim_end_matches('/'), name))
}

fn sftp_home(sftp: &Sftp) -> Option<String> {
    sftp.realpath(Path::new("."))
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

fn sftp_download_file(sftp: &Sftp, remote: &str, local: &Path) -> Result<(), String> {
    let mut remote_file = sftp
        .open(Path::new(remote))
        .map_err(|e| format!("open '{remote}': {e}"))?;
    if let Some(parent) = local.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut local_file = fs::File::create(local)
        .map_err(|e| format!("create '{}': {e}", local.display()))?;
    // Stream with a fixed buffer — never loads the whole file into memory.
    std::io::copy(&mut remote_file, &mut local_file)
        .map_err(|e| format!("download '{remote}': {e}"))?;
    Ok(())
}

fn sftp_download_dir(sftp: &Sftp, remote: &str, local: &Path) -> Result<usize, String> {
    fs::create_dir_all(local).map_err(|e| e.to_string())?;
    let mut count = 0;
    let readdir = sftp
        .readdir(Path::new(remote))
        .map_err(|e| format!("list '{remote}': {e}"))?;
    for (entry_path, stat) in readdir {
        let name = entry_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() || name == "." || name == ".." {
            continue;
        }
        let child_remote = join_remote(remote, &name);
        let child_local = local.join(&name);
        if stat.is_dir() {
            count += sftp_download_dir(sftp, &child_remote, &child_local)?;
        } else {
            sftp_download_file(sftp, &child_remote, &child_local)?;
            count += 1;
        }
    }
    Ok(count)
}

fn sftp_upload_file(sftp: &Sftp, local: &Path, remote: &str) -> Result<(), String> {
    let mut local_file =
        fs::File::open(local).map_err(|e| format!("open '{}': {e}", local.display()))?;
    if let Some(parent) = Path::new(remote).parent() {
        sftp_mkdir_recursive(sftp, &parent.to_string_lossy());
    }
    let mut remote_file = sftp
        .create(Path::new(remote))
        .map_err(|e| format!("create '{remote}': {e}"))?;
    // Stream with a fixed buffer — never loads the whole file into memory.
    std::io::copy(&mut local_file, &mut remote_file)
        .map_err(|e| format!("upload '{remote}': {e}"))?;
    Ok(())
}

fn sftp_upload_dir(sftp: &Sftp, local: &Path, remote: &str) -> Result<usize, String> {
    sftp_mkdir_recursive(sftp, remote);
    let mut count = 0;
    for entry in fs::read_dir(local).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let child_remote = join_remote(remote, &name);
        let path = entry.path();
        if path.is_dir() {
            count += sftp_upload_dir(sftp, &path, &child_remote)?;
        } else {
            sftp_upload_file(sftp, &path, &child_remote)?;
            count += 1;
        }
    }
    Ok(count)
}

fn sftp_remove_dir_recursive(sftp: &Sftp, path: &str) -> Result<(), String> {
    let readdir = sftp
        .readdir(Path::new(path))
        .map_err(|e| format!("list '{path}': {e}"))?;
    for (entry_path, stat) in readdir {
        let name = entry_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() || name == "." || name == ".." {
            continue;
        }
        let child = join_remote(path, &name);
        if stat.is_dir() {
            sftp_remove_dir_recursive(sftp, &child)?;
        } else {
            sftp.unlink(Path::new(&child))
                .map_err(|e| format!("delete '{child}': {e}"))?;
        }
    }
    sftp.rmdir(Path::new(path))
        .map_err(|e| format!("rmdir '{path}': {e}"))
}

fn sftp_mkdir_recursive(sftp: &Sftp, path: &str) {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut current = String::new();
    for part in parts {
        current = format!("{current}/{part}");
        sftp.mkdir(Path::new(&current), 0o755).ok();
    }
}
