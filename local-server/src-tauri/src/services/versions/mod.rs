mod types;
mod cache;
mod fallback;
mod php;
mod nginx;
mod mariadb;
mod nodejs;
mod python;
mod bun;
mod apache;

pub use types::ServiceVersion;
use cache::VersionCacheManager;
use tauri::AppHandle;

pub struct VersionFetcher;

impl VersionFetcher {
    pub async fn fetch_php_versions(app: &AppHandle, force_refresh: bool) -> Result<Vec<ServiceVersion>, String> {
        if !force_refresh {
            if let Some(cached) = VersionCacheManager::get(app, "php").await {
                return Ok(cached);
            }
        }

        match php::fetch_from_windows_php().await {
            Ok(versions) => {
                let _ = VersionCacheManager::set(app, "php", versions.clone()).await;
                Ok(versions)
            }
            Err(e) => {
                log::warn!("PHP API fetch failed: {}, using fallback", e);
                Ok(php::get_fallback())
            }
        }
    }

    pub async fn fetch_nginx_versions(app: &AppHandle, force_refresh: bool) -> Result<Vec<ServiceVersion>, String> {
        if !force_refresh {
            if let Some(cached) = VersionCacheManager::get(app, "nginx").await {
                return Ok(cached);
            }
        }

        match nginx::fetch_from_nginx_org().await {
            Ok(versions) => {
                let _ = VersionCacheManager::set(app, "nginx", versions.clone()).await;
                Ok(versions)
            }
            Err(e) => {
                log::warn!("Nginx fetch failed: {}, using fallback", e);
                Ok(fallback::get_nginx_fallback())
            }
        }
    }

    pub async fn fetch_mariadb_versions(app: &AppHandle, force_refresh: bool) -> Result<Vec<ServiceVersion>, String> {
        if !force_refresh {
            if let Some(cached) = VersionCacheManager::get(app, "mariadb").await {
                return Ok(cached);
            }
        }

        match mariadb::fetch_from_mariadb_api().await {
            Ok(versions) => {
                let _ = VersionCacheManager::set(app, "mariadb", versions.clone()).await;
                Ok(versions)
            }
            Err(e) => {
                log::warn!("MariaDB API fetch failed: {}, using fallback", e);
                Ok(fallback::get_mariadb_fallback())
            }
        }
    }

    pub async fn fetch_nodejs_versions(app: &AppHandle, force_refresh: bool) -> Result<Vec<ServiceVersion>, String> {
        if !force_refresh {
            if let Some(cached) = VersionCacheManager::get(app, "nodejs").await {
                return Ok(cached);
            }
        }

        match nodejs::fetch_from_nodejs_org().await {
            Ok(versions) => {
                let _ = VersionCacheManager::set(app, "nodejs", versions.clone()).await;
                Ok(versions)
            }
            Err(e) => {
                log::warn!("Node.js fetch failed: {}, using fallback", e);
                Ok(nodejs::get_fallback())
            }
        }
    }

    pub async fn fetch_python_versions(app: &AppHandle, force_refresh: bool) -> Result<Vec<ServiceVersion>, String> {
        if !force_refresh {
            if let Some(cached) = VersionCacheManager::get(app, "python").await {
                return Ok(cached);
            }
        }

        match python::fetch_from_endoflife().await {
            Ok(versions) => {
                let _ = VersionCacheManager::set(app, "python", versions.clone()).await;
                Ok(versions)
            }
            Err(e) => {
                log::warn!("Python fetch failed: {}, using fallback", e);
                Ok(python::get_fallback())
            }
        }
    }

    pub async fn fetch_bun_versions(app: &AppHandle, force_refresh: bool) -> Result<Vec<ServiceVersion>, String> {
        if !force_refresh {
            if let Some(cached) = VersionCacheManager::get(app, "bun").await {
                return Ok(cached);
            }
        }

        match bun::fetch_from_github().await {
            Ok(versions) => {
                let _ = VersionCacheManager::set(app, "bun", versions.clone()).await;
                Ok(versions)
            }
            Err(e) => {
                log::warn!("Bun fetch failed: {}, using fallback", e);
                Ok(bun::get_fallback())
            }
        }
    }

    pub async fn fetch_apache_versions(app: &AppHandle, force_refresh: bool) -> Result<Vec<ServiceVersion>, String> {
        if !force_refresh {
            if let Some(cached) = VersionCacheManager::get(app, "apache").await {
                return Ok(cached);
            }
        }

        match apache::fetch_from_apache_lounge().await {
            Ok(versions) => {
                let _ = VersionCacheManager::set(app, "apache", versions.clone()).await;
                Ok(versions)
            }
            Err(e) => {
                log::warn!("Apache fetch failed: {}, using fallback", e);
                Ok(apache::get_fallback())
            }
        }
    }

    pub async fn refresh_all(app: &AppHandle) -> Result<(), String> {
        let _ = VersionCacheManager::clear_all(app).await;

        let (php_result, nginx_result, mariadb_result, nodejs_result, python_result, bun_result, apache_result) = tokio::join!(
            Self::fetch_php_versions(app, true),
            Self::fetch_nginx_versions(app, true),
            Self::fetch_mariadb_versions(app, true),
            Self::fetch_nodejs_versions(app, true),
            Self::fetch_python_versions(app, true),
            Self::fetch_bun_versions(app, true),
            Self::fetch_apache_versions(app, true),
        );

        if let Err(e) = php_result {
            log::error!("PHP refresh failed: {}", e);
        }
        if let Err(e) = nginx_result {
            log::error!("Nginx refresh failed: {}", e);
        }
        if let Err(e) = mariadb_result {
            log::error!("MariaDB refresh failed: {}", e);
        }
        if let Err(e) = nodejs_result {
            log::error!("Node.js refresh failed: {}", e);
        }
        if let Err(e) = python_result {
            log::error!("Python refresh failed: {}", e);
        }
        if let Err(e) = bun_result {
            log::error!("Bun refresh failed: {}", e);
        }
        if let Err(e) = apache_result {
            log::error!("Apache refresh failed: {}", e);
        }

        Ok(())
    }
}
