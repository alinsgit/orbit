mod types;
mod cache;

pub use types::ServiceVersion;
use types::VersionSource;
use cache::VersionCacheManager;
use crate::services::registry::LibraryRegistry;
use tauri::AppHandle;

pub struct VersionFetcher;

impl VersionFetcher {
    /// Fetch versions for any service from the registry
    pub async fn fetch_versions(app: &AppHandle, service: &str, force_refresh: bool) -> Result<Vec<ServiceVersion>, String> {
        // Check cache first
        if !force_refresh {
            if let Some(cached) = VersionCacheManager::get(app, service).await {
                return Ok(cached);
            }
        }

        // Fetch registry (remote with fallback)
        let registry = if force_refresh {
            LibraryRegistry::fetch().await.unwrap_or_else(|_| {
                LibraryRegistry::load_fallback().unwrap_or_else(|e| panic!("Fallback registry broken: {}", e))
            })
        } else {
            LibraryRegistry::get().await.unwrap_or_else(|_| {
                LibraryRegistry::load_fallback().unwrap_or_else(|e| panic!("Fallback registry broken: {}", e))
            })
        };

        // Convert registry data to ServiceVersion[]
        let versions = Self::registry_to_versions(&registry, service)?;

        // Cache result
        let _ = VersionCacheManager::set(app, service, versions.clone()).await;
        Ok(versions)
    }

    /// Refresh all service versions
    pub async fn refresh_all(app: &AppHandle) -> Result<(), String> {
        let _ = VersionCacheManager::clear_all(app).await;

        let services = ["php", "nginx", "apache", "mariadb", "nodejs", "python", "bun"];
        let mut errors = Vec::new();

        for service in &services {
            if let Err(e) = Self::fetch_versions(app, service, true).await {
                log::error!("{} refresh failed: {}", service, e);
                errors.push(format!("{}: {}", service, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            // Still Ok - partial failures are expected
            log::warn!("Some services failed to refresh: {:?}", errors);
            Ok(())
        }
    }

    /// Convert registry ServiceInfo to Vec<ServiceVersion>
    fn registry_to_versions(registry: &LibraryRegistry, service: &str) -> Result<Vec<ServiceVersion>, String> {
        let service_info = registry.services.get(service)
            .ok_or_else(|| format!("Service '{}' not found in registry", service))?;

        let platform = Self::get_current_platform();
        let mut versions = Vec::new();

        // Multi-version service (php, mariadb, nodejs, python, nginx, bun, apache)
        if let Some(version_map) = &service_info.versions {
            let keys: Vec<String> = if let Some(available) = &service_info.available_versions {
                available.clone()
            } else {
                version_map.keys().cloned().collect()
            };

            for ver_key in &keys {
                if let Some(ver_info) = version_map.get(ver_key) {
                    if let Some(dl) = Self::get_platform_download_from_version(ver_info, &platform) {
                        // Use version key if it differs from latest (e.g. Apache "2.4.66-VS18")
                        let display_version = if ver_key != &ver_info.latest {
                            ver_key.clone()
                        } else {
                            ver_info.latest.clone()
                        };
                        versions.push(ServiceVersion {
                            version: display_version,
                            download_url: dl.url.clone(),
                            filename: dl.filename.clone(),
                            release_date: None,
                            source: VersionSource::Api,
                        });
                    }
                }
            }
        } else {
            // Single version service (redis, mailpit, composer)
            if let Some(latest) = &service_info.latest {
                if let Some(dl) = Self::get_platform_download_from_service(service_info, &platform) {
                    versions.push(ServiceVersion {
                        version: latest.clone(),
                        download_url: dl.url.clone(),
                        filename: dl.filename.clone(),
                        release_date: None,
                        source: VersionSource::Api,
                    });
                }
            }
        }

        if versions.is_empty() {
            return Err(format!("No versions found for '{}' on platform '{}'", service, platform));
        }

        Ok(versions)
    }

    fn get_current_platform() -> String {
        #[cfg(target_os = "windows")]
        return "windows".to_string();
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "macos_arm64".to_string();
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "macos_x64".to_string();
        #[cfg(target_os = "linux")]
        return "linux".to_string();
    }

    fn get_platform_download_from_version<'a>(
        version: &'a crate::services::registry::VersionInfo,
        platform: &str,
    ) -> Option<&'a crate::services::registry::PlatformDownload> {
        match platform {
            "windows" => version.windows.as_ref(),
            "macos_arm64" => version.macos_arm64.as_ref(),
            "macos_x64" => version.macos_x64.as_ref(),
            "linux" => version.linux.as_ref(),
            _ => version.all_platforms.as_ref(),
        }
    }

    fn get_platform_download_from_service<'a>(
        service: &'a crate::services::registry::ServiceInfo,
        platform: &str,
    ) -> Option<&'a crate::services::registry::PlatformDownload> {
        match platform {
            "windows" => service.windows.as_ref(),
            "macos_arm64" => service.macos_arm64.as_ref(),
            "macos_x64" => service.macos_x64.as_ref(),
            "linux" => service.linux.as_ref(),
            _ => service.all_platforms.as_ref(),
        }
    }
}
