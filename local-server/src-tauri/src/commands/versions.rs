use tauri::{command, AppHandle};
use crate::services::versions::{VersionFetcher, ServiceVersion};

#[command]
pub async fn get_available_versions(
    app: AppHandle,
    service: String,
    force_refresh: Option<bool>,
) -> Result<Vec<ServiceVersion>, String> {
    let force = force_refresh.unwrap_or(false);

    match service.as_str() {
        "php" => VersionFetcher::fetch_php_versions(&app, force).await,
        "nginx" => VersionFetcher::fetch_nginx_versions(&app, force).await,
        "mariadb" => VersionFetcher::fetch_mariadb_versions(&app, force).await,
        "nodejs" => VersionFetcher::fetch_nodejs_versions(&app, force).await,
        "python" => VersionFetcher::fetch_python_versions(&app, force).await,
        "bun" => VersionFetcher::fetch_bun_versions(&app, force).await,
        "apache" => VersionFetcher::fetch_apache_versions(&app, force).await,
        _ => Err("Unknown service type".to_string()),
    }
}

#[command]
pub async fn refresh_all_versions(app: AppHandle) -> Result<(), String> {
    VersionFetcher::refresh_all(&app).await
}
