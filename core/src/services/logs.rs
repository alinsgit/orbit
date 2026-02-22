use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogReadResult {
    pub entries: Vec<LogEntry>,
    pub total_lines: usize,
    pub filtered_lines: usize,
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

        // MariaDB error log (data lives in bin/data/mariadb/, not bin/mariadb/data/)
        let mariadb_log = bin_path.join("data").join("mariadb").join("mysql.err");
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

        // Apache logs
        let apache_logs = bin_path.join("apache").join("logs");
        if apache_logs.exists() {
            if let Ok(entries) = fs::read_dir(&apache_logs) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "log").unwrap_or(false) {
                        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let metadata = fs::metadata(&path).ok();

                        let log_type = if name.contains("error") {
                            "apache-error"
                        } else if name.contains("access") {
                            "apache-access"
                        } else {
                            "apache"
                        };

                        logs.push(LogFile {
                            name: format!("apache-{}", name),
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

        // PostgreSQL log (stderr log in data directory)
        let pg_data = bin_path.join("data").join("postgres");
        if pg_data.exists() {
            // PostgreSQL on Windows logs to pg_log/ or log/ directory if configured
            for log_subdir in &["pg_log", "log"] {
                let pg_log_dir = pg_data.join(log_subdir);
                if pg_log_dir.exists() {
                    if let Ok(entries) = fs::read_dir(&pg_log_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                            if ext == "log" || ext == "csv" {
                                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                let metadata = fs::metadata(&path).ok();
                                logs.push(LogFile {
                                    name: format!("postgresql-{}", name),
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
                                    log_type: "postgresql".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // MongoDB log
        let mongodb_data = bin_path.join("data").join("mongodb");
        if mongodb_data.exists() {
            // Check for mongod.log in data dir or bin dir
            for log_dir in &[&mongodb_data, &bin_path.join("mongodb")] {
                let mongo_log = log_dir.join("mongod.log");
                if mongo_log.exists() {
                    let metadata = fs::metadata(&mongo_log).ok();
                    logs.push(LogFile {
                        name: "mongodb.log".to_string(),
                        path: mongo_log.to_string_lossy().to_string(),
                        size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                        modified: metadata
                            .and_then(|m| m.modified().ok())
                            .map(|t| {
                                chrono::DateTime::<chrono::Utc>::from(t)
                                    .format("%Y-%m-%d %H:%M:%S")
                                    .to_string()
                            })
                            .unwrap_or_default(),
                        log_type: "mongodb".to_string(),
                    });
                    break;
                }
            }
        }

        // Sort by modified date (newest first)
        logs.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(logs)
    }

    /// Check if a log line matches the given filters
    fn matches_filters(line: &str, level_filter: Option<&str>, search_query: Option<&str>) -> bool {
        if let Some(level) = level_filter {
            if level != "all" {
                let detected = Self::detect_log_level(line);
                if detected != level {
                    return false;
                }
            }
        }
        if let Some(query) = search_query {
            if !query.is_empty() && !line.to_lowercase().contains(&query.to_lowercase()) {
                return false;
            }
        }
        true
    }

    /// Read log file with parsing, server-side filtering, and pagination.
    /// Uses a ring buffer to avoid loading entire large files into memory.
    pub fn read_log(
        path: &str,
        lines: usize,
        offset: usize,
        level_filter: Option<&str>,
        search_query: Option<&str>,
    ) -> Result<LogReadResult, String> {
        let path = PathBuf::from(path);
        if !path.exists() {
            return Ok(LogReadResult {
                entries: vec![LogEntry {
                    timestamp: None,
                    level: "info".to_string(),
                    message: "Log file not found".to_string(),
                    raw: "Log file not found".to_string(),
                }],
                total_lines: 0,
                filtered_lines: 0,
            });
        }

        let file = File::open(&path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);

        let has_filters = level_filter.map_or(false, |l| l != "all")
            || search_query.map_or(false, |q| !q.is_empty());

        let needed = offset + lines;
        let mut ring: VecDeque<String> = VecDeque::with_capacity(needed + 1);
        let mut total_lines: usize = 0;
        let mut filtered_lines: usize = 0;

        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => continue,
            };

            if line.trim().is_empty() {
                continue;
            }

            total_lines += 1;

            if has_filters && !Self::matches_filters(&line, level_filter, search_query) {
                continue;
            }

            filtered_lines += 1;
            ring.push_back(line);
            if ring.len() > needed {
                ring.pop_front();
            }
        }

        // If no filters active, filtered_lines equals total_lines
        if !has_filters {
            filtered_lines = total_lines;
        }

        // ring contains the last `needed` (or fewer) matching lines
        let ring_len = ring.len();
        let take_end = if ring_len > offset { ring_len - offset } else { 0 };
        let take_start = if take_end > lines { take_end - lines } else { 0 };

        let entries: Vec<LogEntry> = ring
            .range(take_start..take_end)
            .rev()
            .map(|line| Self::parse_log_line(line))
            .collect();

        Ok(LogReadResult {
            entries,
            total_lines,
            filtered_lines,
        })
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

        // PostgreSQL log: LOG, ERROR, FATAL, PANIC, WARNING, NOTICE
        if line.contains("FATAL:") || line.contains("PANIC:") {
            return "error";
        }
        if line.contains("ERROR:") && !line.contains("[ERROR]") {
            // Avoid double-matching MariaDB [ERROR]
            return "error";
        }
        if line.contains("WARNING:") && !line.contains("[Warning]") {
            return "warning";
        }

        // Apache error log: [level:severity] format (Apache 2.4+)
        if line.contains("[error]") || line.contains("[crit]") || line.contains("[alert]") || line.contains("[emerg]") {
            // Already caught above by nginx pattern (same bracket format)
        }
        if line.contains(":error]") || line.contains(":crit]") || line.contains(":alert]") || line.contains(":emerg]") {
            return "error";
        }
        if line.contains(":warn]") {
            return "warning";
        }

        // MongoDB log: severity codes S (fatal), E (error), W (warning), I (info)
        if line.contains(" E ") || line.contains(" F ") {
            return "error";
        }
        if line.contains(" W ") {
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
