use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Cache service types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheType {
    Redis,
    Memcached,
}

/// Cache service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub cache_type: CacheType,
    pub port: u16,
    pub max_memory: String, // e.g., "128mb", "256mb"
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_type: CacheType::Redis,
            port: 6379,
            max_memory: "128mb".to_string(),
            enabled: false,
        }
    }
}

/// Cache service status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatus {
    pub redis_installed: bool,
    pub redis_path: Option<String>,
    pub redis_running: bool,
    pub redis_port: u16,
    pub memcached_installed: bool,
    pub memcached_path: Option<String>,
    pub memcached_running: bool,
    pub memcached_port: u16,
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

    /// Check if Memcached is installed
    pub fn is_memcached_installed(app: &AppHandle) -> Result<bool, String> {
        let bin_dir = Self::get_cache_dir(app)?;
        let memcached_exe = bin_dir.join("memcached").join("memcached.exe");
        Ok(memcached_exe.exists())
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

    /// Get Memcached path
    pub fn get_memcached_path(app: &AppHandle) -> Result<Option<PathBuf>, String> {
        let bin_dir = Self::get_cache_dir(app)?;
        let memcached_dir = bin_dir.join("memcached");
        if memcached_dir.exists() {
            Ok(Some(memcached_dir))
        } else {
            Ok(None)
        }
    }

    /// Check if Redis is running
    pub fn is_redis_running() -> bool {
        Self::check_port_in_use(6379)
    }

    /// Check if Memcached is running
    pub fn is_memcached_running() -> bool {
        Self::check_port_in_use(11211)
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
            memcached_installed: Self::is_memcached_installed(app)?,
            memcached_path: Self::get_memcached_path(app)?.map(|p| p.to_string_lossy().to_string()),
            memcached_running: Self::is_memcached_running(),
            memcached_port: 11211,
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

    /// Create Memcached start script
    pub fn create_memcached_config(app: &AppHandle, config: &CacheConfig) -> Result<(), String> {
        let memcached_dir = Self::get_cache_dir(app)?.join("memcached");
        if !memcached_dir.exists() {
            return Err("Memcached is not installed".to_string());
        }

        // Parse max memory to MB
        let max_mb = Self::parse_memory_to_mb(&config.max_memory);

        // Create a batch file for easy starting
        let batch_content = format!(
            r#"@echo off
cd /d "%~dp0"
memcached.exe -p {} -m {} -l 127.0.0.1
"#,
            config.port, max_mb
        );

        let batch_path = memcached_dir.join("start.bat");
        fs::write(&batch_path, batch_content)
            .map_err(|e| format!("Failed to write Memcached start script: {}", e))?;

        Ok(())
    }

    /// Parse memory string to MB (e.g., "128mb" -> 128)
    fn parse_memory_to_mb(memory: &str) -> u32 {
        let memory = memory.to_lowercase();
        if memory.ends_with("gb") {
            memory
                .trim_end_matches("gb")
                .parse::<u32>()
                .unwrap_or(1)
                * 1024
        } else if memory.ends_with("mb") {
            memory.trim_end_matches("mb").parse::<u32>().unwrap_or(128)
        } else {
            memory.parse::<u32>().unwrap_or(128)
        }
    }

    /// Get Redis download URL
    pub fn get_redis_download_url() -> &'static str {
        // Redis for Windows (Microsoft archive or alternatives)
        "https://github.com/tporadowski/redis/releases/download/v5.0.14.1/Redis-x64-5.0.14.1.zip"
    }

    /// Get Memcached download URL
    pub fn get_memcached_download_url() -> &'static str {
        // Memcached for Windows
        "https://static.runoob.com/download/memcached-1.4.5-amd64.zip"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory() {
        assert_eq!(CacheManager::parse_memory_to_mb("128mb"), 128);
        assert_eq!(CacheManager::parse_memory_to_mb("256MB"), 256);
        assert_eq!(CacheManager::parse_memory_to_mb("1gb"), 1024);
        assert_eq!(CacheManager::parse_memory_to_mb("2GB"), 2048);
    }
}
