use std::process::{Child, Command};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::os::windows::process::CommandExt;

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
        path.push("ngrok.exe");
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
        let _ = Command::new(&bin_path)
            .arg("config")
            .arg("add-authtoken")
            .arg(auth_token)
            .creation_flags(CREATE_NO_WINDOW)
            .output(); // wait for token to be saved

        // Start ngrok tunnel mapping to our domain (using specified host-header to pass to Nginx correctly)
        // ngrok http <port> --host-header=<domain>
        let child = Command::new(&bin_path)
            .arg("http")
            .arg(port.to_string())
            .arg(format!("--host-header={}", domain))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to spawn Ngrok: {}", e))?;

        *child_guard = Some(child);

        // Allow some time for the process to spin up the local API (usually instant, wait 1.5 secs)
        std::thread::sleep(std::time::Duration::from_millis(1500));

        // Let the frontend know we started Successfully; frontend will fetch URL from get_tunnel_url
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
                Err(e) => Err(format!("Failed to kill Ngrok process: {}", e)),
            }
        } else {
            Ok("No active tunnel found to stop.".to_string())
        }
    }
}
