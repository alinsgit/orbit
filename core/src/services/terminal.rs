use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::io::Write;
use std::io::Read;
use tokio::sync::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter, State};

// Maintain active terminal sessions by ID
pub struct TerminalState {
    pub ptys: Arc<Mutex<HashMap<String, Box<dyn MasterPty + Send>>>>,
    pub writers: Arc<Mutex<HashMap<String, Box<dyn std::io::Write + Send>>>>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            ptys: Arc::new(Mutex::new(HashMap::new())),
            writers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub fn build_orbit_path(_app_handle: &AppHandle) -> String {
    let app_dir = crate::services::paths::get_orbit_data_dir();
    let bin_dir = app_dir.join("bin");
    
    // We want the most specific binaries in front.
    let mut paths_to_inject = vec![
        bin_dir.join("mariadb").join("bin"),
        bin_dir.join("mariadb"),
        bin_dir.join("nginx"),
        bin_dir.join("nodejs"),
        bin_dir.join("bun"),
        bin_dir.join("go").join("bin"),
        bin_dir.join("deno"),
        bin_dir.join("python"),
        bin_dir.join("apache").join("bin"),
        bin_dir.join("composer"),
        bin_dir.join("phpmyadmin"),
        bin_dir.join("tools"), // Catch-all
    ];

    // Push all installed PHP versions into the path list dynamically
    if let Ok(entries) = std::fs::read_dir(bin_dir.join("php")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                paths_to_inject.push(path);
            }
        }
    }
    
    // Convert to strings and filter valid
    let mut custom_paths: Vec<String> = paths_to_inject
        .into_iter()
        .filter(|p| p.exists())
        .filter_map(|p| p.to_str().map(|s| s.to_string()))
        .collect();
        
    // Append existing system PATH
    if let Ok(system_path) = env::var("PATH") {
        custom_paths.push(system_path);
    }
    
    let separator = if cfg!(windows) { ";" } else { ":" };
    custom_paths.join(separator)
}

#[tauri::command]
pub async fn spawn_terminal(
    app_handle: AppHandle,
    state: State<'_, TerminalState>,
    id: String,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
) -> Result<(), String> {
    let pty_system = native_pty_system();
    
    let pty_pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| format!("Failed to open pty: {e}"))?;
    
    // Determinate default shell
    let shell_fallback = "bash".to_string();
    let shell_env = env::var("SHELL").unwrap_or_else(|_| shell_fallback.clone());
    
    let shell_path = if cfg!(windows) {
        if env::var("PSModulePath").is_ok() { "powershell.exe" } else { "cmd.exe" }
    } else {
        &shell_env
    };
    
    let mut cmd = CommandBuilder::new(shell_path);
    
    // Inject custom path
    let injected_path = build_orbit_path(&app_handle);
    cmd.env("PATH", &injected_path);
    
    // Set CWD to user's specified dir, or fall back to www root
    if let Some(cwd_path) = cwd {
        let path = std::path::PathBuf::from(cwd_path);
        if path.exists() {
            cmd.cwd(&path);
        }
    } else {
        let app_dir = crate::services::paths::get_orbit_data_dir();
        let default_cwd = app_dir.join("www"); // Try to launch in www folder
        if default_cwd.exists() {
            cmd.cwd(&default_cwd);
        }
    }
    
    let _child = pty_pair.slave.spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {e}"))?;
        
    // Save the master side to state so we can write/resize later
    let master = pty_pair.master;
    
    let master_reader = master.try_clone_reader().map_err(|e| e.to_string())?;
    let master_writer = master.take_writer().map_err(|e| e.to_string())?;
    
    let mut ptys = state.ptys.lock().await;
    ptys.insert(id.clone(), master);
    
    let mut writers = state.writers.lock().await;
    writers.insert(id.clone(), master_writer);
    
    // Drop locks
    drop(ptys);
    drop(writers);
    
    // Spawn reader thread
    let app_clone = app_handle.clone();
    let id_clone = id.clone();
    
    std::thread::spawn(move || {
        let mut reader = master_reader;
        let mut buf = [0; 1024];
        
        loop {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let text = String::from_utf8_lossy(&buf[0..n]).to_string();
                    let _ = app_clone.emit(&format!("pty-output-{id_clone}"), text);
                }
                _ => break, // EOF or Error
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn write_terminal(
    state: State<'_, TerminalState>,
    id: String,
    data: String,
) -> Result<(), String> {
    if let Some(writer) = state.writers.lock().await.get_mut(&id) {
        writer.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Terminal session not found".into())
    }
}

#[tauri::command]
pub async fn resize_terminal(
    state: State<'_, TerminalState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    if let Some(master) = state.ptys.lock().await.get(&id) {
        master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Terminal session not found".into())
    }
}

#[tauri::command]
pub async fn close_terminal(
    state: State<'_, TerminalState>,
    id: String,
) -> Result<(), String> {
    state.writers.lock().await.remove(&id);
    state.ptys.lock().await.remove(&id);
    Ok(())
}
