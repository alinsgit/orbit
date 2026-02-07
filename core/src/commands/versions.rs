use tauri::{command, AppHandle};
use crate::services::versions::{VersionFetcher, ServiceVersion};

#[command]
pub async fn get_available_versions(
    app: AppHandle,
    service: String,
    force_refresh: Option<bool>,
) -> Result<Vec<ServiceVersion>, String> {
    let force = force_refresh.unwrap_or(false);
    VersionFetcher::fetch_versions(&app, &service, force).await
}

#[command]
pub async fn refresh_all_versions(app: AppHandle) -> Result<(), String> {
    VersionFetcher::refresh_all(&app).await
}
