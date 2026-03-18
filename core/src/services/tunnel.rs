use std::process::{Child, Command};
use std::sync::Mutex;
use once_cell::sync::Lazy;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Since Ngrok tunnels block the thread, we will store the Child process handle
// so we can kill it when the user stops the tunnel via UI. 
static NGROK_PROCESS: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

pub struct TunnelManager {
    base_dir: std::path::PathBuf,
}

impl TunnelManager {
    pub fn new(base_dir: std::path::PathBuf) -> Self {
        Self { base_dir }
    }

    fn get_ngrok_bin(&self) -> std::path::PathBuf {
        let mut path = self.base_dir.clone();
        path.push("bin");
        path.push("ngrok");
        if cfg!(windows) {
            path.push("ngrok.exe");
        } else {
            path.push("ngrok");
        }
        path
    }

    pub fn start_tunnel(&self, domain: &str, port: u16, auth_token: &str) -> Result<String, String> {
        let bin_path = self.get_ngrok_bin();
        if !bin_path.exists() {
            return Err("Ngrok is not installed in the Orbit environment. Please install it from the Services tab.".to_string());
        }

        let mut child_guard = NGROK_PROCESS.lock().unwrap();

        // If an instance is already running, kill it before starting a new one.
        if let Some(mut existing_child) = child_guard.take() {
            let _ = existing_child.kill();
            let _ = existing_child.wait();
        }

        // Configure Auth Token first
        let mut auth_cmd = Command::new(&bin_path);
        auth_cmd.arg("config")
            .arg("add-authtoken")
            .arg(auth_token);
        #[cfg(windows)]
        auth_cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = auth_cmd.output();

        // Start ngrok tunnel mapping to our domain
        let mut tunnel_cmd = Command::new(&bin_path);
        tunnel_cmd.arg("http")
            .arg(port.to_string())
            .arg(format!("--host-header={domain}"));
        #[cfg(windows)]
        tunnel_cmd.creation_flags(CREATE_NO_WINDOW);
        
        let child = tunnel_cmd.spawn()
            .map_err(|e| format!("Failed to spawn Ngrok: {e}"))?;

        *child_guard = Some(child);

        // Allow some time for the process to spin up the local API
        std::thread::sleep(std::time::Duration::from_millis(1500));

        Ok("Tunnel successfully initiated.".to_string())
    }

    pub fn stop_tunnel(&self) -> Result<String, String> {
        let mut child_guard = NGROK_PROCESS.lock().unwrap();
        
        if let Some(mut existing_child) = child_guard.take() {
            match existing_child.kill() {
                Ok(_) => {
                    let _ = existing_child.wait();
                    Ok("Tunnel successfully stopped.".to_string())
                },
                Err(e) => Err(format!("Failed to kill Ngrok process: {e}")),
            }
        } else {
            Ok("No active tunnel found to stop.".to_string())
        }
    }
}
