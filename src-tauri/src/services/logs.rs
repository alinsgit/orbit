use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: String,
    pub log_type: String, // "access", "error", "php"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: Option<String>,
    pub level: String, // "info", "warning", "error"
    pub message: String,
    pub raw: String,
}

pub struct LogManager;

impl LogManager {
    /// Get all available log files
    pub fn get_log_files(bin_path: &PathBuf) -> Result<Vec<LogFile>, String> {
        let mut logs = Vec::new();

        // Nginx logs
        let nginx_logs = bin_path.join("nginx").join("logs");
        if nginx_logs.exists() {
            if let Ok(entries) = fs::read_dir(&nginx_logs) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "log").unwrap_or(false) {
                        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let metadata = fs::metadata(&path).ok();

                        let log_type = if name.contains("error") {
                            "error"
                        } else if name.contains("access") {
                            "access"
                        } else {
                            "other"
                        };

                        logs.push(LogFile {
                            name: name.clone(),
                            path: path.to_string_lossy().to_string(),
                            size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                            modified: metadata
                                .and_then(|m| m.modified().ok())
                                .map(|t| {
                                    chrono::DateTime::<chrono::Utc>::from(t)
                                        .format("%Y-%m-%d %H:%M:%S")
                                        .to_string()
                                })
                                .unwrap_or_default(),
                            log_type: log_type.to_string(),
                        });
                    }
                }
            }
        }

        // PHP error log (if configured)
        let php_dirs = ["8.4", "8.5"];
        for version in php_dirs {
            let php_log = bin_path.join("php").join(version).join("logs").join("php_errors.log");
            if php_log.exists() {
                let metadata = fs::metadata(&php_log).ok();
                logs.push(LogFile {
                    name: format!("php-{}-errors.log", version),
                    path: php_log.to_string_lossy().to_string(),
                    size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                    modified: metadata
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            chrono::DateTime::<chrono::Utc>::from(t)
                                .format("%Y-%m-%d %H:%M:%S")
                                .to_string()
                        })
                        .unwrap_or_default(),
                    log_type: "php".to_string(),
                });
            }
        }

        // MariaDB error log
        let mariadb_log = bin_path.join("mariadb").join("data").join("mysql.err");
        if mariadb_log.exists() {
            let metadata = fs::metadata(&mariadb_log).ok();
            logs.push(LogFile {
                name: "mariadb-error.log".to_string(),
                path: mariadb_log.to_string_lossy().to_string(),
                size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        chrono::DateTime::<chrono::Utc>::from(t)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                    })
                    .unwrap_or_default(),
                log_type: "mariadb".to_string(),
            });
        }

        // Sort by modified date (newest first)
        logs.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(logs)
    }

    /// Read log file with parsing
    pub fn read_log(path: &str, lines: usize, offset: usize) -> Result<Vec<LogEntry>, String> {
        let path = PathBuf::from(path);
        if !path.exists() {
            return Ok(vec![LogEntry {
                timestamp: None,
                level: "info".to_string(),
                message: "Log file not found".to_string(),
                raw: "Log file not found".to_string(),
            }]);
        }

        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let all_lines: Vec<&str> = content.lines().collect();

        // Get lines from end, with offset
        let total = all_lines.len();
        let start = if total > offset + lines {
            total - offset - lines
        } else {
            0
        };
        let end = if total > offset { total - offset } else { 0 };

        let entries: Vec<LogEntry> = all_lines[start..end]
            .iter()
            .rev()
            .map(|line| Self::parse_log_line(line))
            .collect();

        Ok(entries)
    }

    /// Read raw log file (last N lines)
    pub fn read_raw_log(path: &str, lines: usize) -> Result<Vec<String>, String> {
        let path = PathBuf::from(path);
        if !path.exists() {
            return Ok(vec!["Log file not found".to_string()]);
        }

        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let all_lines: Vec<String> = content
            .lines()
            .rev()
            .take(lines)
            .map(|s| s.to_string())
            .collect();

        Ok(all_lines)
    }

    /// Parse a log line to extract timestamp and level
    fn parse_log_line(line: &str) -> LogEntry {
        let level = if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
            "error"
        } else if line.contains("warn") || line.contains("Warn") || line.contains("WARNING") {
            "warning"
        } else {
            "info"
        };

        // Try to extract timestamp (common nginx format: 2024/01/01 12:00:00)
        let timestamp = if line.len() > 20 {
            // Nginx error log format: 2024/01/01 12:00:00 [error] ...
            if line.chars().nth(4) == Some('/') && line.chars().nth(7) == Some('/') {
                Some(line[..19].to_string())
            }
            // Access log format: IP - - [01/Jan/2024:12:00:00 +0000]
            else if let Some(start) = line.find('[') {
                if let Some(end) = line.find(']') {
                    Some(line[start + 1..end].to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        LogEntry {
            timestamp,
            level: level.to_string(),
            message: line.to_string(),
            raw: line.to_string(),
        }
    }

    /// Clear a log file
    pub fn clear_log(path: &str) -> Result<(), String> {
        fs::write(path, "").map_err(|e| format!("Failed to clear log: {}", e))
    }

    /// Get log file size
    pub fn get_log_size(path: &str) -> Result<u64, String> {
        let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
        Ok(metadata.len())
    }

    // Legacy methods for backward compatibility
    pub fn read_nginx_log(log_path: &PathBuf, lines: usize) -> Result<Vec<String>, String> {
        Self::read_raw_log(&log_path.to_string_lossy(), lines)
    }

    pub fn read_mariadb_log(log_path: &PathBuf, lines: usize) -> Result<Vec<String>, String> {
        Self::read_raw_log(&log_path.to_string_lossy(), lines)
    }
}
