use serde::Serialize;
use tauri::{AppHandle, Manager};
use crate::services::tunnel::TunnelManager;

#[derive(Serialize)]
pub struct TunnelResponse {
    pub success: bool,
    pub message: String,
    pub url: Option<String>,
}

#[derive(serde::Deserialize)]
struct NgrokApiTunnel {
    public_url: String,
    proto: String,
}

#[derive(serde::Deserialize)]
struct NgrokApiTunnelsResponse {
    tunnels: Vec<NgrokApiTunnel>,
}

#[tauri::command]
pub fn start_tunnel(
    domain: String,
    port: u16,
    auth_token: String,
    app: AppHandle,
) -> Result<TunnelResponse, String> {
    let base_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let manager = TunnelManager::new(base_dir);
    
    match manager.start_tunnel(&domain, port, &auth_token) {
        Ok(msg) => Ok(TunnelResponse {
            success: true,
            message: msg,
            url: None, // Frontend will poll `get_tunnel_url` to get the generated local endpoint
        }),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn stop_tunnel(app: AppHandle) -> Result<TunnelResponse, String> {
    let base_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let manager = TunnelManager::new(base_dir);
    
    match manager.stop_tunnel() {
        Ok(msg) => Ok(TunnelResponse {
            success: true,
            message: msg,
            url: None,
        }),
        Err(e) => Err(e),
    }
}

// Interrogates the local Ngrok API (running on port 4040) to find the public URL
#[tauri::command]
pub async fn get_tunnel_url() -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client.get("http://127.0.0.1:4040/api/tunnels")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to local Ngrok API: {e}"))?;
        
    let tunnels_response: NgrokApiTunnelsResponse = res.json().await
        .map_err(|e| format!("Failed to parse Ngrok API response: {e}"))?;
        
    // Look for HTTPS tunnel specifically
    if let Some(https_tunnel) = tunnels_response.tunnels.iter().find(|t| t.proto == "https") {
        return Ok(https_tunnel.public_url.clone());
    }
    
    // Fallback to whichever is first
    if let Some(first_tunnel) = tunnels_response.tunnels.first() {
        return Ok(first_tunnel.public_url.clone());
    }

    Err("No active tunnels found on local Ngrok instance".to_string())
}
