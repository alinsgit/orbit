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

        // PHP error logs - scan all installed PHP versions dynamically
        let php_base = bin_path.join("php");
        if php_base.exists() {
            if let Ok(entries) = fs::read_dir(&php_base) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let version = entry.file_name().to_string_lossy().to_string();
                        let logs_dir = entry.path().join("logs");
                        let php_log = logs_dir.join("php_errors.log");
                        // Ensure logs dir and file exist
                        if !logs_dir.exists() {
                            let _ = fs::create_dir_all(&logs_dir);
                        }
                        if !php_log.exists() {
                            let _ = fs::write(&php_log, "");
                        }
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
                }
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

        // Mailpit log - show if mailpit is installed
        let mailpit_dir = bin_path.join("mailpit");
        let mailpit_log = mailpit_dir.join("mailpit.log");
        if mailpit_dir.exists() {
            // Ensure log file exists so it appears in sidebar
            if !mailpit_log.exists() {
                let _ = fs::write(&mailpit_log, "");
            }
        }
        if mailpit_log.exists() {
            let metadata = fs::metadata(&mailpit_log).ok();
            logs.push(LogFile {
                name: "mailpit.log".to_string(),
                path: mailpit_log.to_string_lossy().to_string(),
                size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        chrono::DateTime::<chrono::Utc>::from(t)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                    })
                    .unwrap_or_default(),
                log_type: "mailpit".to_string(),
            });
        }

        // Redis log - show if redis is installed
        let redis_dir = bin_path.join("redis");
        let redis_log = redis_dir.join("redis.log");
        if redis_dir.exists() {
            if !redis_log.exists() {
                let _ = fs::write(&redis_log, "");
            }
        }
        if redis_log.exists() {
            let metadata = fs::metadata(&redis_log).ok();
            logs.push(LogFile {
                name: "redis.log".to_string(),
                path: redis_log.to_string_lossy().to_string(),
                size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        chrono::DateTime::<chrono::Utc>::from(t)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                    })
                    .unwrap_or_default(),
                log_type: "redis".to_string(),
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


    /// Detect log level from line content using format-specific patterns
    fn detect_log_level(line: &str) -> &'static str {
        // Nginx error log bracket format: [error], [crit], [alert], [emerg], [warn], [notice]
        if line.contains("[crit]") || line.contains("[alert]") || line.contains("[emerg]") || line.contains("[error]") {
            return "error";
        }
        if line.contains("[warn]") {
            return "warning";
        }
        if line.contains("[notice]") || line.contains("[info]") {
            return "info";
        }

        // PHP log format
        if line.contains("PHP Fatal error") || line.contains("PHP Parse error") {
            return "error";
        }
        if line.contains("PHP Warning") {
            return "warning";
        }
        if line.contains("PHP Notice") || line.contains("PHP Deprecated") {
            return "info";
        }

        // MariaDB log format: [ERROR], [Warning], [Note]
        if line.contains("[ERROR]") {
            return "error";
        }
        if line.contains("[Warning]") {
            return "warning";
        }
        if line.contains("[Note]") {
            return "info";
        }

        // Mailpit structured log: level=error, level=warn, level=info
        if line.contains("level=error") || line.contains("level=fatal") {
            return "error";
        }
        if line.contains("level=warn") {
            return "warning";
        }
        if line.contains("level=info") || line.contains("level=debug") {
            return "info";
        }

        // Redis log: symbols after timestamp indicate level
        // # = warning, * = info/notice, - = verbose
        if line.contains(" # ") {
            return "warning";
        }

        // Nginx access log: extract HTTP status code
        // Format: IP - user [timestamp] "METHOD /path HTTP/x.x" STATUS SIZE ...
        if let Some(req_start) = line.find("] \"") {
            let after_open = &line[req_start + 3..];
            if let Some(req_end) = after_open.find("\" ") {
                let after_req = &after_open[req_end + 2..];
                if after_req.len() >= 3 && after_req.as_bytes()[..3].iter().all(|b| b.is_ascii_digit()) {
                    if let Ok(status) = after_req[..3].parse::<u16>() {
                        if status >= 500 {
                            return "error";
                        }
                        if status >= 400 {
                            return "warning";
                        }
                        return "info";
                    }
                }
            }
        }

        "info"
    }

    /// Extract timestamp from various log formats
    fn extract_timestamp(line: &str) -> Option<String> {
        if line.len() < 10 {
            return None;
        }

        // Nginx error log: 2024/01/01 12:00:00 [level] ...
        if line.len() >= 19
            && line.as_bytes().get(4) == Some(&b'/')
            && line.as_bytes().get(7) == Some(&b'/')
        {
            return Some(line[..19].to_string());
        }

        // MariaDB log: 2024-01-01 12:00:00 ...
        if line.len() >= 19
            && line.as_bytes().get(4) == Some(&b'-')
            && line.as_bytes().get(7) == Some(&b'-')
            && line.as_bytes().get(10) == Some(&b' ')
        {
            return Some(line[..19].to_string());
        }

        // Access log / PHP log: [...timestamp...]
        if let Some(start) = line.find('[') {
            if let Some(end) = line[start..].find(']') {
                return Some(line[start + 1..start + end].to_string());
            }
        }

        None
    }

    /// Extract message content by stripping timestamp and level prefix
    fn extract_message(line: &str) -> String {
        // Nginx error log: "2024/01/01 12:00:00 [error] 1234#0: message"
        if line.len() >= 19
            && line.as_bytes().get(4) == Some(&b'/')
            && line.as_bytes().get(7) == Some(&b'/')
        {
            // Find closing bracket of level, then skip "PID#TID: "
            if let Some(bracket_end) = line.find("] ") {
                return line[bracket_end + 2..].to_string();
            }
        }

        // PHP log: "[timestamp] PHP Fatal error: message"
        if let Some(bracket_end) = line.find("] ") {
            let rest = &line[bracket_end + 2..];
            if rest.starts_with("PHP ") {
                return rest.to_string();
            }
        }

        // MariaDB: "2024-01-01 12:00:00 0 [Note] message"
        if line.len() >= 19
            && line.as_bytes().get(4) == Some(&b'-')
            && line.as_bytes().get(7) == Some(&b'-')
        {
            if let Some(bracket_end) = line.find("] ") {
                return line[bracket_end + 2..].to_string();
            }
        }

        line.to_string()
    }

    /// Parse a log line to extract timestamp, level, and message
    fn parse_log_line(line: &str) -> LogEntry {
        let level = Self::detect_log_level(line);
        let timestamp = Self::extract_timestamp(line);
        let message = Self::extract_message(line);

        LogEntry {
            timestamp,
            level: level.to_string(),
            message,
            raw: line.to_string(),
        }
    }

    /// Clear a log file
    pub fn clear_log(path: &str) -> Result<(), String> {
        fs::write(path, "").map_err(|e| format!("Failed to clear log: {}", e))
    }

}
