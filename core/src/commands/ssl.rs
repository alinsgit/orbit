use crate::services::ssl::{SSLManager, SslCertificate, SslStatus};
use tauri::{command, AppHandle, Manager};

fn get_bin_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())
        .map(|p| p.join("bin"))
}

#[command]
pub fn get_ssl_status(app: AppHandle) -> Result<SslStatus, String> {
    let bin_path = get_bin_path(&app)?;
    Ok(SSLManager::get_status(&bin_path))
}

#[command]
pub async fn install_mkcert(app: AppHandle) -> Result<String, String> {
    let bin_path = get_bin_path(&app)?;
    SSLManager::install_mkcert(&bin_path).await
}

#[command]
pub fn install_ssl_ca(app: AppHandle) -> Result<String, String> {
    let bin_path = get_bin_path(&app)?;
    SSLManager::install_ca(&bin_path)
}

#[command]
pub fn generate_ssl_cert(app: AppHandle, domain: String) -> Result<SslCertificate, String> {
    let bin_path = get_bin_path(&app)?;
    SSLManager::generate_cert(&bin_path, &domain)
}

#[command]
pub fn get_ssl_cert(app: AppHandle, domain: String) -> Result<Option<SslCertificate>, String> {
    let bin_path = get_bin_path(&app)?;
    Ok(SSLManager::get_cert(&bin_path, &domain))
}

#[command]
pub fn list_ssl_certs(app: AppHandle) -> Result<Vec<SslCertificate>, String> {
    let bin_path = get_bin_path(&app)?;
    Ok(SSLManager::list_certs(&bin_path))
}

#[command]
pub fn delete_ssl_cert(app: AppHandle, domain: String) -> Result<(), String> {
    let bin_path = get_bin_path(&app)?;
    SSLManager::delete_cert(&bin_path, &domain)
}
