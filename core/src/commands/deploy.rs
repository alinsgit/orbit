use crate::services::deploy::{DeployManifest, DeployService};
use crate::services::deploy_store::{DeployStore, DeployTarget, ServerConnection};
use tauri::{command, AppHandle};

// ── Global Connections ──

#[command]
pub fn deploy_list_connections(app: AppHandle) -> Result<Vec<ServerConnection>, String> {
    DeployStore::list_connections(&app)
}

#[command]
pub fn deploy_add_connection(
    app: AppHandle,
    connection: ServerConnection,
    password: Option<String>,
) -> Result<String, String> {
    DeployStore::add_connection(&app, connection, password)?;
    Ok("Connection added".to_string())
}

#[command]
pub fn deploy_remove_connection(app: AppHandle, name: String) -> Result<String, String> {
    DeployStore::remove_connection(&app, &name)?;
    Ok("Connection removed".to_string())
}

#[command]
pub fn deploy_test_connection(app: AppHandle, name: String) -> Result<String, String> {
    let conn = DeployStore::get_connection(&app, &name)?
        .ok_or_else(|| format!("Connection not found: {name}"))?;
    DeployService::test_connection(&conn)
}

// ── Site Targets ──

#[command]
pub fn deploy_list_targets(app: AppHandle, domain: String) -> Result<Vec<DeployTarget>, String> {
    DeployStore::list_targets(&app, &domain)
}

#[command]
pub fn deploy_assign_target(
    app: AppHandle,
    domain: String,
    connection: String,
    remote_path: String,
) -> Result<String, String> {
    DeployStore::assign_target(
        &app,
        &domain,
        DeployTarget {
            connection,
            remote_path,
        },
    )?;
    Ok("Target assigned".to_string())
}

#[command]
pub fn deploy_unassign_target(
    app: AppHandle,
    domain: String,
    connection: String,
) -> Result<String, String> {
    DeployStore::unassign_target(&app, &domain, &connection)?;
    Ok("Target unassigned".to_string())
}

// ── Operations ──

#[command]
pub fn deploy_sync(
    app: AppHandle,
    domain: String,
    connection: String,
    site_path: String,
) -> Result<DeployManifest, String> {
    let conn = DeployStore::get_connection(&app, &connection)?
        .ok_or_else(|| format!("Connection not found: {connection}"))?;

    // Get remote_path from target
    let targets = DeployStore::list_targets(&app, &domain)?;
    let target = targets
        .iter()
        .find(|t| t.connection == connection)
        .ok_or_else(|| format!("No deploy target for connection '{connection}' on '{domain}'"))?;

    let path = std::path::Path::new(&site_path);
    DeployService::sync(&app, &domain, &conn, &target.remote_path, path)
}

#[command]
pub fn deploy_ssh_execute(
    app: AppHandle,
    connection: String,
    command: String,
) -> Result<String, String> {
    let conn = DeployStore::get_connection(&app, &connection)?
        .ok_or_else(|| format!("Connection not found: {connection}"))?;
    DeployService::ssh_execute(&conn, &command)
}

#[command]
pub fn deploy_get_status(
    app: AppHandle,
    domain: String,
    connection: String,
) -> Result<Option<DeployManifest>, String> {
    DeployService::get_last_manifest(&app, &domain, &connection)
}
