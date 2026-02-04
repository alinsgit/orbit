use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use super::types::{CachedVersions, ServiceVersion, VersionCache, VersionSource};

const CACHE_KEY: &str = "version_cache";
const CACHE_TTL_SECS: i64 = 86400; // 24 hours

pub struct VersionCacheManager;

impl VersionCacheManager {
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    pub fn is_valid(cached: &CachedVersions) -> bool {
        let now = Self::now();
        now - cached.fetched_at < CACHE_TTL_SECS
    }

    pub async fn get(app: &AppHandle, service: &str) -> Option<Vec<ServiceVersion>> {
        let store = app.store(CACHE_KEY).ok()?;
        let json_value = store.get("data")?;
        let cache: VersionCache = serde_json::from_value(json_value).ok()?;

        let cached = match service {
            "php" => cache.php,
            "nginx" => cache.nginx,
            "mariadb" => cache.mariadb,
            _ => None,
        }?;

        if Self::is_valid(&cached) {
            let mut versions = cached.versions;
            for v in &mut versions {
                v.source = VersionSource::Cache;
            }
            Some(versions)
        } else {
            None
        }
    }

    pub async fn set(app: &AppHandle, service: &str, versions: Vec<ServiceVersion>) -> Result<(), String> {
        let store = app.store(CACHE_KEY).map_err(|e| e.to_string())?;

        let mut cache: VersionCache = store
            .get("data")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let cached = CachedVersions {
            versions,
            fetched_at: Self::now(),
            service: service.to_string(),
        };

        match service {
            "php" => cache.php = Some(cached),
            "nginx" => cache.nginx = Some(cached),
            "mariadb" => cache.mariadb = Some(cached),
            _ => return Err("Unknown service".to_string()),
        }

        let json_value = serde_json::to_value(&cache).map_err(|e| e.to_string())?;
        store.set("data", json_value);
        store.save().map_err(|e| e.to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn clear(app: &AppHandle, service: &str) -> Result<(), String> {
        let store = app.store(CACHE_KEY).map_err(|e| e.to_string())?;

        let mut cache: VersionCache = store
            .get("data")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        match service {
            "php" => cache.php = None,
            "nginx" => cache.nginx = None,
            "mariadb" => cache.mariadb = None,
            _ => return Err("Unknown service".to_string()),
        }

        let json_value = serde_json::to_value(&cache).map_err(|e| e.to_string())?;
        store.set("data", json_value);
        store.save().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn clear_all(app: &AppHandle) -> Result<(), String> {
        let store = app.store(CACHE_KEY).map_err(|e| e.to_string())?;
        let json_value = serde_json::to_value(&VersionCache::default()).map_err(|e| e.to_string())?;
        store.set("data", json_value);
        store.save().map_err(|e| e.to_string())?;
        Ok(())
    }
}
