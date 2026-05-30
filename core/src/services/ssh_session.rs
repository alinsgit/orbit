// Shared SSH session infrastructure for the Remote workspace:
//   • a per-connection SFTP session pool (responsive file browsing — no
//     handshake+auth on every listdir like the one-shot deploy helpers)
//   • an interactive SSH PTY engine (persistent shell channel owned by a
//     single thread, driven via an mpsc command queue — the SSH analogue of
//     services/terminal.rs)
//
// ssh2::Channel is NOT Sync, so the shell channel is owned by exactly one
// thread; writes/resizes are delivered to it through a channel. SFTP sessions
// are wrapped in Arc<Mutex<Session>> so different connections never block each
// other while one is mid-transfer.

use crate::services::deploy_store::{AuthMethod, DeployStore, ServerConnection};
use ssh2::Session;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// ─── Session Factory ─────────────────────────────────────────────────

/// Open and authenticate a fresh SSH session for a connection.
/// Shared by both the SFTP pool and the interactive terminal.
pub fn create_session(conn: &ServerConnection) -> Result<Session, String> {
    let addr = format!("{}:{}", conn.host, conn.port);
    let tcp = TcpStream::connect(&addr).map_err(|e| format!("Connection failed: {e}"))?;
    let mut session = Session::new().map_err(|e| format!("SSH error: {e}"))?;
    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| format!("Handshake failed: {e}"))?;

    // Verify the server's host key (TOFU) BEFORE sending any credentials.
    verify_host_key(&session, &conn.host, conn.port)?;

    match &conn.auth {
        AuthMethod::Password => {
            let password = DeployStore::get_password(&conn.name)?;
            session
                .userauth_password(&conn.username, &password)
                .map_err(|e| format!("Auth failed: {e}"))?;
        }
        AuthMethod::KeyFile(path) => {
            session
                .userauth_pubkey_file(&conn.username, None, Path::new(path), None)
                .map_err(|e| format!("Key auth failed: {e}"))?;
        }
    }

    if !session.authenticated() {
        return Err("Authentication failed".to_string());
    }

    // Keep idle connections alive so pooled SFTP sessions and interactive
    // shells aren't dropped by the server's idle timeout — the "connection
    // clogs after a while" symptom.
    session.set_keepalive(true, 30);

    Ok(session)
}

/// Trust-on-first-use host-key verification against Orbit's *own*
/// `~/.orbit/known_hosts`. Blocks the connection only on a definitive key
/// MISMATCH (changed key / possible MITM); unknown hosts are pinned on first
/// use. Failures to initialize the checker never block (best-effort), so the
/// guarantee matches a normal SSH client's TOFU model.
///
/// We deliberately do NOT touch the user's `~/.ssh/known_hosts`: libssh2's
/// `write_file` rewrites the whole file from its in-memory parse and silently
/// drops entry types it doesn't understand (certificates, `sk-` keys, CA
/// markers), which would corrupt the file the user's CLI relies on and cause
/// spurious "host key changed" errors. A separate Orbit file keeps the two
/// independent.
fn verify_host_key(session: &Session, host: &str, port: u16) -> Result<(), String> {
    let mut known = match session.known_hosts() {
        Ok(k) => k,
        Err(_) => return Ok(()),
    };

    let kh_path = known_hosts_path();
    if let Some(ref p) = kh_path {
        known.read_file(p, ssh2::KnownHostFileKind::OpenSSH).ok();
    }

    let (key, key_type) = match session.host_key() {
        Some(k) => k,
        None => return Ok(()),
    };

    match known.check_port(host, port, key) {
        ssh2::CheckResult::Match => Ok(()),
        ssh2::CheckResult::Mismatch => Err(format!(
            "Host key MISMATCH for {host}:{port} — possible man-in-the-middle. \
             If the server key legitimately changed, remove the old entry from \
             your ~/.orbit/known_hosts and reconnect."
        )),
        // Unknown host (or transient check failure): trust on first use + pin.
        ssh2::CheckResult::NotFound | ssh2::CheckResult::Failure => {
            // Can't pin a key whose type ssh2 doesn't recognize.
            if matches!(key_type, ssh2::HostKeyType::Unknown) {
                return Ok(());
            }
            if let Some(p) = kh_path {
                let add_host = if port == 22 {
                    host.to_string()
                } else {
                    format!("[{host}]:{port}")
                };
                let fmt: ssh2::KnownHostKeyFormat = key_type.into();
                if known.add(&add_host, key, "orbit", fmt).is_ok() {
                    if let Some(parent) = p.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    known.write_file(&p, ssh2::KnownHostFileKind::OpenSSH).ok();
                }
            }
            Ok(())
        }
    }
}

fn known_hosts_path() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|h| std::path::PathBuf::from(h).join(".orbit").join("known_hosts"))
}

// ─── SFTP Connection Pool ────────────────────────────────────────────

/// Caches one authenticated SSH session per connection name so file-browsing
/// operations reuse the existing handshake. Each session is individually
/// locked so concurrent operations on different connections don't serialize.
#[derive(Default)]
pub struct SshConnPool {
    sessions: Mutex<HashMap<String, Arc<Mutex<Session>>>>,
}

impl SshConnPool {
    /// Return the pooled session for `name`, opening (and caching) one if
    /// absent. The connection metadata is loaded from the deploy store.
    ///
    /// A cached session is reused only after a liveness probe succeeds — a
    /// dead pooled session (server idle-timeout, network blip) would otherwise
    /// make every subsequent file operation fail forever, the "connection
    /// clogs" symptom. A failed probe evicts the entry and reconnects.
    pub fn get_or_connect(
        &self,
        app: &AppHandle,
        name: &str,
    ) -> Result<Arc<Mutex<Session>>, String> {
        {
            let map = self.sessions.lock().map_err(|_| "pool poisoned")?;
            if let Some(s) = map.get(name) {
                let alive = s
                    .lock()
                    .map(|sess| sess.keepalive_send().is_ok())
                    .unwrap_or(false);
                if alive {
                    return Ok(s.clone());
                }
            }
        }
        // Absent or stale — drop any dead entry, then connect fresh.
        self.disconnect(name);

        let conn = DeployStore::get_connection(app, name)?
            .ok_or_else(|| format!("Connection not found: {name}"))?;
        let session = create_session(&conn)?;
        let arc = Arc::new(Mutex::new(session));

        let mut map = self.sessions.lock().map_err(|_| "pool poisoned")?;
        // Another thread may have raced us — keep whichever landed first.
        Ok(map.entry(name.to_string()).or_insert(arc).clone())
    }

    /// Run `f` against the pooled session, evicting the session if it fails so
    /// the next call reconnects. Centralizes the get → lock → operate pattern
    /// and guarantees a half-open connection (which the up-front liveness probe
    /// can miss) never stays cached after an operation error.
    pub fn with_session<T>(
        &self,
        app: &AppHandle,
        name: &str,
        f: impl FnOnce(&Session) -> Result<T, String>,
    ) -> Result<T, String> {
        let arc = self.get_or_connect(app, name)?;
        let result = {
            let sess = arc.lock().map_err(|_| "session poisoned")?;
            f(&sess)
        };
        if result.is_err() {
            self.disconnect(name);
        }
        result
    }

    /// Drop a pooled session (e.g. user disconnected or it went stale).
    pub fn disconnect(&self, name: &str) {
        if let Ok(mut map) = self.sessions.lock() {
            map.remove(name);
        }
    }
}

// ─── Interactive SSH PTY ─────────────────────────────────────────────

/// Commands delivered to the thread that owns a live shell channel.
pub enum TermCmd {
    Write(Vec<u8>),
    Resize(u16, u16),
    Close,
}

/// Tracks active SSH shell sessions by id (mirrors TerminalState for local PTYs).
#[derive(Default)]
pub struct SshTerminalState {
    senders: Mutex<HashMap<String, Sender<TermCmd>>>,
}

impl SshTerminalState {
    fn insert(&self, id: String, tx: Sender<TermCmd>) {
        if let Ok(mut map) = self.senders.lock() {
            map.insert(id, tx);
        }
    }

    fn send(&self, id: &str, cmd: TermCmd) -> Result<(), String> {
        let map = self.senders.lock().map_err(|_| "ssh term state poisoned")?;
        let tx = map
            .get(id)
            .ok_or_else(|| "SSH terminal session not found".to_string())?;
        tx.send(cmd).map_err(|_| "SSH terminal closed".to_string())
    }

    fn remove(&self, id: &str) {
        if let Ok(mut map) = self.senders.lock() {
            map.remove(id);
        }
    }
}

/// Write the full buffer to a non-blocking channel, retrying on EAGAIN.
fn write_all_nonblocking(channel: &mut ssh2::Channel, data: &[u8]) -> std::io::Result<()> {
    let mut written = 0;
    while written < data.len() {
        match channel.write(&data[written..]) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "ssh channel write returned 0",
                ))
            }
            Ok(n) => written += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Spawn an interactive shell on a fresh session for `conn`, streaming output
/// over the `ssh-pty-output-{id}` event and accepting input/resize via the
/// returned command queue (stored in `state`).
pub fn spawn_ssh_terminal(
    app: AppHandle,
    state: &SshTerminalState,
    conn: &ServerConnection,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let session = create_session(conn)?;

    let mut channel = session
        .channel_session()
        .map_err(|e| format!("Channel error: {e}"))?;
    channel
        .request_pty(
            "xterm-256color",
            None,
            Some((cols as u32, rows as u32, 0, 0)),
        )
        .map_err(|e| format!("PTY request failed: {e}"))?;
    channel
        .shell()
        .map_err(|e| format!("Shell start failed: {e}"))?;

    // Non-blocking so the owner thread can interleave reads and queued writes.
    session.set_blocking(false);

    let (tx, rx): (Sender<TermCmd>, Receiver<TermCmd>) = std::sync::mpsc::channel();
    state.insert(id.clone(), tx);

    let output_event = format!("ssh-pty-output-{id}");
    let closed_event = format!("ssh-pty-closed-{id}");

    std::thread::spawn(move || {
        // Move the session in so it outlives the channel for the thread's life.
        let _session = session;
        let mut buf = [0u8; 8192];

        loop {
            // Drain queued commands first.
            let mut should_close = false;
            loop {
                match rx.try_recv() {
                    Ok(TermCmd::Write(data)) => {
                        if write_all_nonblocking(&mut channel, &data).is_err() {
                            should_close = true;
                            break;
                        }
                        let _ = channel.flush();
                    }
                    Ok(TermCmd::Resize(c, r)) => {
                        channel
                            .request_pty_size(c as u32, r as u32, None, None)
                            .ok();
                    }
                    Ok(TermCmd::Close) => {
                        should_close = true;
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        should_close = true;
                        break;
                    }
                }
            }
            if should_close {
                break;
            }

            // Read whatever output is available.
            match channel.read(&mut buf) {
                Ok(0) => {
                    if channel.eof() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app.emit(&output_event, text);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if channel.eof() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
                Err(_) => break,
            }
        }

        channel.close().ok();
        channel.wait_close().ok();
        let _ = app.emit(&closed_event, ());
    });

    Ok(())
}

pub fn write_ssh_terminal(state: &SshTerminalState, id: &str, data: String) -> Result<(), String> {
    state.send(id, TermCmd::Write(data.into_bytes()))
}

pub fn resize_ssh_terminal(
    state: &SshTerminalState,
    id: &str,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    state.send(id, TermCmd::Resize(cols, rows))
}

pub fn close_ssh_terminal(state: &SshTerminalState, id: &str) -> Result<(), String> {
    // Best-effort signal; thread also exits when the sender is dropped.
    state.send(id, TermCmd::Close).ok();
    state.remove(id);
    Ok(())
}
