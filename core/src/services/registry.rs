/**
 * Library Registry Service
 * 
 * Fetches and caches library information from orbit-libraries repository.
 * This provides centralized version management for all downloadable services.
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use once_cell::sync::Lazy;

/// Registry URL - points to GitHub raw content
const REGISTRY_URL: &str = "https://raw.githubusercontent.com/alinsgit/orbit-libraries/main/dist/libraries.json";

/// Fallback/offline registry (embedded at compile time)
const FALLBACK_REGISTRY: &str = include_str!("../../dist/libraries.json");

/// Global cached registry
static REGISTRY_CACHE: Lazy<RwLock<Option<LibraryRegistry>>> = Lazy::new(|| RwLock::new(None));

/// Platform-specific download info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformDownload {
    pub url: String,
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// Version info for services with multiple versions (PHP, MariaDB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub latest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows: Option<PlatformDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macos_arm64: Option<PlatformDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macos_x64: Option<PlatformDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linux: Option<PlatformDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_platforms: Option<PlatformDownload>,
}

/// Service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "availableVersions")]
    pub available_versions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<HashMap<String, VersionInfo>>,
    // Single-version services
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows: Option<PlatformDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macos_arm64: Option<PlatformDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macos_x64: Option<PlatformDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linux: Option<PlatformDownload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_platforms: Option<PlatformDownload>,
}

/// Root library registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryRegistry {
    #[serde(rename = "$schema")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub updated: String,
    pub version: String,
    pub services: HashMap<String, ServiceInfo>,
}

impl LibraryRegistry {
    /// Load registry from remote URL with fallback
    pub async fn fetch() -> Result<Self, String> {
        let registry = match Self::fetch_remote().await {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Remote registry unavailable: {}, using fallback", e);
                Self::load_fallback()?
            }
        };

        // Cache regardless of source (remote or fallback)
        if let Ok(mut cache) = REGISTRY_CACHE.write() {
            *cache = Some(registry.clone());
        }
        Ok(registry)
    }

    /// Fetch from remote URL
    async fn fetch_remote() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create client: {}", e))?;

        let response = client
            .get(REGISTRY_URL)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch registry: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Registry fetch failed: {}", response.status()));
        }

        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse registry: {}", e))
    }

    /// Load from embedded fallback
    pub fn load_fallback() -> Result<Self, String> {
        serde_json::from_str(FALLBACK_REGISTRY)
            .map_err(|e| format!("Failed to parse fallback registry: {}", e))
    }

    /// Get cached registry or fetch
    pub async fn get() -> Result<Self, String> {
        // Check cache first
        if let Ok(cache) = REGISTRY_CACHE.read() {
            if let Some(ref registry) = *cache {
                return Ok(registry.clone());
            }
        }
        
        // Fetch if not cached
        Self::fetch().await
    }

    /// Get download URL for a service
    pub fn get_download_url(&self, service: &str, version: Option<&str>) -> Option<String> {
        let service_info = self.services.get(service)?;
        
        // Get platform-specific download
        let platform = Self::get_current_platform();
        
        // Check if it's a multi-version service
        if let Some(versions) = &service_info.versions {
            let ver = version.unwrap_or_else(|| {
                service_info.available_versions.as_ref()
                    .and_then(|v| v.first())
                    .map(|s| s.as_str())
                    .unwrap_or("latest")
            });
            
            let version_info = versions.get(ver)?;
            return Self::get_platform_url(version_info, &platform);
        }
        
        // Single version service
        Self::get_platform_url_from_service(service_info, &platform)
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

    fn get_platform_url(version: &VersionInfo, platform: &str) -> Option<String> {
        match platform {
            "windows" => version.windows.as_ref().map(|d| d.url.clone()),
            "macos_arm64" => version.macos_arm64.as_ref().map(|d| d.url.clone()),
            "macos_x64" => version.macos_x64.as_ref().map(|d| d.url.clone()),
            "linux" => version.linux.as_ref().map(|d| d.url.clone()),
            _ => version.all_platforms.as_ref().map(|d| d.url.clone()),
        }
    }

    fn get_platform_url_from_service(service: &ServiceInfo, platform: &str) -> Option<String> {
        match platform {
            "windows" => service.windows.as_ref().map(|d| d.url.clone()),
            "macos_arm64" => service.macos_arm64.as_ref().map(|d| d.url.clone()),
            "macos_x64" => service.macos_x64.as_ref().map(|d| d.url.clone()),
            "linux" => service.linux.as_ref().map(|d| d.url.clone()),
            _ => service.all_platforms.as_ref().map(|d| d.url.clone()),
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_fallback() {
        let registry = LibraryRegistry::load_fallback().unwrap();
        assert!(!registry.services.is_empty());
        assert!(registry.services.contains_key("php"));
        assert!(registry.services.contains_key("nginx"));
    }

    #[test]
    fn test_get_download_url() {
        let registry = LibraryRegistry::load_fallback().unwrap();
        
        // Test nginx
        let url = registry.get_download_url("nginx", None);
        assert!(url.is_some());
        assert!(url.unwrap().contains("nginx"));
        
        // Test PHP with version
        let url = registry.get_download_url("php", Some("8.3"));
        assert!(url.is_some());
    }
}
