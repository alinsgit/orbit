use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Cache service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub port: u16,
    pub max_memory: String, // e.g., "128mb", "256mb"
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            port: 6379,
            max_memory: "128mb".to_string(),
            enabled: false,
        }
    }
}

/// Cache service status (Redis only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatus {
    pub redis_installed: bool,
    pub redis_path: Option<String>,
    pub redis_running: bool,
    pub redis_port: u16,
}

pub struct CacheManager;

impl CacheManager {
    /// Get cache directory
    fn get_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let bin_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin");
        Ok(bin_dir)
    }

    /// Check if Redis is installed
    pub fn is_redis_installed(app: &AppHandle) -> Result<bool, String> {
        let bin_dir = Self::get_cache_dir(app)?;
        let redis_exe = bin_dir.join("redis").join("redis-server.exe");
        Ok(redis_exe.exists())
    }

    /// Get Redis path
    pub fn get_redis_path(app: &AppHandle) -> Result<Option<PathBuf>, String> {
        let bin_dir = Self::get_cache_dir(app)?;
        let redis_dir = bin_dir.join("redis");
        if redis_dir.exists() {
            Ok(Some(redis_dir))
        } else {
            Ok(None)
        }
    }

    /// Check if Redis is running
    pub fn is_redis_running() -> bool {
        Self::check_port_in_use(6379)
    }

    /// Check if a port is in use
    fn check_port_in_use(port: u16) -> bool {
        use std::net::TcpListener;
        TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
    }

    /// Get full cache status
    pub fn get_status(app: &AppHandle) -> Result<CacheStatus, String> {
        Ok(CacheStatus {
            redis_installed: Self::is_redis_installed(app)?,
            redis_path: Self::get_redis_path(app)?.map(|p| p.to_string_lossy().to_string()),
            redis_running: Self::is_redis_running(),
            redis_port: 6379,
        })
    }

    /// Create Redis configuration file
    pub fn create_redis_config(app: &AppHandle, config: &CacheConfig) -> Result<(), String> {
        let redis_dir = Self::get_cache_dir(app)?.join("redis");
        if !redis_dir.exists() {
            return Err("Redis is not installed".to_string());
        }

        let config_content = format!(
            r#"# Redis configuration
bind 127.0.0.1
port {}
maxmemory {}
maxmemory-policy allkeys-lru
appendonly no
save ""
loglevel notice
logfile "redis.log"
"#,
            config.port, config.max_memory
        );

        let config_path = redis_dir.join("redis.conf");
        fs::write(&config_path, config_content)
            .map_err(|e| format!("Failed to write Redis config: {}", e))?;

        Ok(())
    }

    /// Get Redis download URL from registry with fallback
    pub async fn get_redis_download_url() -> String {
        // Try to get from registry
        if let Ok(registry) = crate::services::registry::LibraryRegistry::get().await {
            if let Some(url) = registry.get_download_url("redis", None) {
                return url;
            }
        }
        // Fallback to hardcoded URL
        "https://github.com/redis-windows/redis-windows/releases/download/8.4.0/Redis-8.4.0-Windows-x64-cygwin-with-Service.zip".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert_eq!(config.port, 6379);
        assert_eq!(config.max_memory, "128mb");
    }
}
