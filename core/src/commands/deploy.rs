use crate::services::deploy::{DeployManifest, DeployService};
use crate::services::deploy_store::{DeployConnection, DeployStore};
use tauri::{command, AppHandle};

#[command]
pub fn deploy_list_connections(
    app: AppHandle,
    domain: String,
) -> Result<Vec<DeployConnection>, String> {
    DeployStore::load_for_site(&app, &domain)
}

#[command]
pub fn deploy_add_connection(
    app: AppHandle,
    domain: String,
    connection: DeployConnection,
    password: Option<String>,
) -> Result<String, String> {
    DeployStore::add_connection(&app, &domain, connection, password)?;
    Ok("Connection added".to_string())
}

#[command]
pub fn deploy_remove_connection(
    app: AppHandle,
    domain: String,
    conn_name: String,
) -> Result<String, String> {
    DeployStore::remove_connection(&app, &domain, &conn_name)?;
    Ok("Connection removed".to_string())
}

#[command]
pub fn deploy_test_connection(
    app: AppHandle,
    domain: String,
    conn_name: String,
) -> Result<String, String> {
    DeployService::test_connection(&app, &domain, &conn_name)
}

#[command]
pub fn deploy_ssh_execute(
    app: AppHandle,
    domain: String,
    conn_name: String,
    command: String,
) -> Result<String, String> {
    DeployService::ssh_execute(&app, &domain, &conn_name, &command)
}

#[command]
pub fn deploy_sync(
    app: AppHandle,
    domain: String,
    conn_name: String,
    site_path: String,
) -> Result<DeployManifest, String> {
    let connections = DeployStore::load_for_site(&app, &domain)?;
    let conn = connections
        .iter()
        .find(|c| c.name == conn_name)
        .ok_or("Connection not found")?;

    let path = std::path::Path::new(&site_path);
    match conn.protocol {
        crate::services::deploy_store::Protocol::SSH
        | crate::services::deploy_store::Protocol::SFTP => {
            DeployService::sync_sftp(&app, &domain, &conn_name, path)
        }
        crate::services::deploy_store::Protocol::FTP => {
            DeployService::sync_ftp(&app, &domain, &conn_name, path)
        }
    }
}

#[command]
pub fn deploy_get_status(
    app: AppHandle,
    domain: String,
    conn_name: String,
) -> Result<Option<DeployManifest>, String> {
    DeployService::get_last_manifest(&app, &domain, &conn_name)
}
