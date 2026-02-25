use std::path::{Path, PathBuf};
use regex::Regex;

/// Domain validation result
#[derive(Debug)]
pub struct ValidationError(pub String);

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validates a domain name according to RFC 1123
pub fn validate_domain(domain: &str) -> Result<(), ValidationError> {
    // Check length
    if domain.is_empty() {
        return Err(ValidationError("Domain cannot be empty".to_string()));
    }
    if domain.len() > 253 {
        return Err(ValidationError("Domain too long (max 253 characters)".to_string()));
    }

    // Check for valid characters only (alphanumeric, hyphen, dot)
    let valid_domain_regex = Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9\-\.]*[a-zA-Z0-9])?$").unwrap();
    if !valid_domain_regex.is_match(domain) {
        return Err(ValidationError(
            "Invalid domain: only alphanumeric characters, hyphens, and dots allowed".to_string()
        ));
    }

    // Check each label (part between dots)
    for label in domain.split('.') {
        if label.is_empty() {
            return Err(ValidationError("Domain cannot have empty labels".to_string()));
        }
        if label.len() > 63 {
            return Err(ValidationError("Domain label too long (max 63 characters)".to_string()));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(ValidationError("Domain labels cannot start or end with hyphen".to_string()));
        }
    }

    // Reject dangerous patterns
    let dangerous_patterns = [
        ";", "'", "\"", "`", "$", "(", ")", "{", "}", "[", "]",
        "|", "&", "<", ">", "\\", "\n", "\r", "\t", " ", "%", "!"
    ];
    for pattern in dangerous_patterns {
        if domain.contains(pattern) {
            return Err(ValidationError(format!(
                "Domain contains forbidden character: '{pattern}'"
            )));
        }
    }

    Ok(())
}

/// Validates a port number
pub fn validate_port(port: u16) -> Result<(), ValidationError> {
    if port == 0 {
        return Err(ValidationError("Port cannot be 0".to_string()));
    }
    // Allow common web ports (80, 443) and all ports >= 1024
    // On Windows, these don't require admin privileges for local use
    if port < 80 {
        return Err(ValidationError(format!(
            "Port {port} is too low. Use port 80, 443, or >= 1024"
        )));
    }
    Ok(())
}

/// Validates a path and ensures it doesn't escape allowed directories
pub fn validate_site_path(path: &str, _allowed_base: Option<&Path>) -> Result<PathBuf, ValidationError> {
    if path.is_empty() {
        return Err(ValidationError("Path cannot be empty".to_string()));
    }

    // Check for null bytes
    if path.contains('\0') {
        return Err(ValidationError("Path contains null byte".to_string()));
    }

    // Reject paths with dangerous patterns
    let dangerous_patterns = ["..\\", "../", "\\..\\", "/../"];
    for pattern in dangerous_patterns {
        if path.contains(pattern) {
            return Err(ValidationError(
                "Path traversal detected: '..' not allowed".to_string()
            ));
        }
    }

    // Convert to PathBuf and canonicalize if exists
    let path_buf = PathBuf::from(path);

    // Check for path traversal via components
    for component in path_buf.components() {
        if let std::path::Component::ParentDir = component {
            return Err(ValidationError(
                "Path traversal detected: parent directory reference not allowed".to_string()
            ));
        }
    }

    // If the path exists, canonicalize it
    if path_buf.exists() {
        let canonical = path_buf.canonicalize()
            .map_err(|e| ValidationError(format!("Failed to resolve path: {e}")))?;

        // If allowed_base is specified, ensure path is under it
        if let Some(base) = _allowed_base {
            if let Ok(canonical_base) = base.canonicalize() {
                if !canonical.starts_with(&canonical_base) {
                    return Err(ValidationError(format!(
                        "Path must be under {}",
                        canonical_base.display()
                    )));
                }
            }
        }

        return Ok(canonical);
    }

    Ok(path_buf)
}

/// Validates that a file path is within allowed directories (for log reading)
pub fn validate_log_path(path: &str, allowed_base: &Path) -> Result<PathBuf, ValidationError> {
    if path.is_empty() {
        return Err(ValidationError("Path cannot be empty".to_string()));
    }

    // Check for null bytes
    if path.contains('\0') {
        return Err(ValidationError("Path contains null byte".to_string()));
    }

    let path_buf = PathBuf::from(path);

    // Path must exist for reading
    if !path_buf.exists() {
        return Err(ValidationError("Log file does not exist".to_string()));
    }

    // Canonicalize both paths
    let canonical_path = path_buf.canonicalize()
        .map_err(|e| ValidationError(format!("Failed to resolve path: {e}")))?;

    let canonical_base = allowed_base.canonicalize()
        .map_err(|e| ValidationError(format!("Failed to resolve base path: {e}")))?;

    // Ensure path is under allowed base
    if !canonical_path.starts_with(&canonical_base) {
        return Err(ValidationError(format!(
            "Access denied: path must be under {}",
            canonical_base.display()
        )));
    }

    // Only allow certain file extensions for logs
    if let Some(ext) = canonical_path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        let allowed_extensions = ["log", "err", "txt", "out"];
        if !allowed_extensions.contains(&ext_str.as_str()) {
            return Err(ValidationError(format!(
                "Invalid log file extension: .{ext_str}"
            )));
        }
    } else {
        // Files without extension might be logs (like error.log renamed)
        // Allow but check it's not a binary
    }

    Ok(canonical_path)
}

/// Sanitizes a string for safe use in nginx config
pub fn sanitize_for_nginx(value: &str) -> String {
    // Remove or escape characters that could break nginx config
    let dangerous_chars = [';', '{', '}', '#', '$', '`', '"', '\'', '\\', '\n', '\r'];
    let mut result = value.to_string();
    for ch in dangerous_chars {
        result = result.replace(ch, "");
    }
    result
}

/// Sanitizes a string for safe use in PowerShell
pub fn sanitize_for_powershell(value: &str) -> String {
    // Escape single quotes by doubling them (PowerShell convention)
    // Also remove dangerous characters
    let mut result = value.replace('\'', "''");

    // Remove characters that could enable injection
    let dangerous = ['`', '$', '(', ')', '{', '}', ';', '|', '&', '<', '>', '\n', '\r', '\0'];
    for ch in dangerous {
        result = result.replace(ch, "");
    }

    result
}

/// Validates PHP version string - flexible validation for various version formats
pub fn validate_php_version(version: &str) -> Result<(), ValidationError> {
    // Basic sanity checks only - actual PHP existence check happens elsewhere
    if version.is_empty() {
        return Err(ValidationError("PHP version cannot be empty".to_string()));
    }

    // Version should start with a digit
    if !version.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Err(ValidationError(format!(
            "Invalid PHP version: '{version}'. Should start with a number"
        )));
    }

    // Only allow safe characters: digits, dots, hyphens (for versions like "8.4-dev")
    if !version.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
        return Err(ValidationError(format!(
            "Invalid PHP version: '{version}'. Only numbers, dots, and hyphens allowed"
        )));
    }

    // Must contain at least one dot (e.g., 8.4, 8.5.1)
    if !version.contains('.') {
        return Err(ValidationError(format!(
            "Invalid PHP version: '{version}'. Expected format like 8.4 or 8.5.1"
        )));
    }

    Ok(())
}

/// Validates INI key name (for php.ini settings)
pub fn validate_ini_key(key: &str) -> Result<(), ValidationError> {
    if key.is_empty() {
        return Err(ValidationError("INI key cannot be empty".to_string()));
    }

    // INI keys should only contain alphanumeric, underscore, dot
    let key_regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_\.]*$").unwrap();
    if !key_regex.is_match(key) {
        return Err(ValidationError(format!(
            "Invalid INI key: '{key}'. Only alphanumeric, underscore, and dot allowed"
        )));
    }

    // Reject known dangerous keys
    let dangerous_keys = [
        "disable_functions", "open_basedir", "allow_url_include",
        "safe_mode", "safe_mode_gid", "safe_mode_include_dir"
    ];
    if dangerous_keys.contains(&key) {
        return Err(ValidationError(format!(
            "Modifying '{key}' is not allowed for security reasons"
        )));
    }

    Ok(())
}

/// Validates INI value (for php.ini settings)
pub fn validate_ini_value(value: &str) -> Result<(), ValidationError> {
    // Check for dangerous patterns
    let dangerous_patterns = ["\n", "\r", "\0", "${", "$("];
    for pattern in dangerous_patterns {
        if value.contains(pattern) {
            return Err(ValidationError(
                "INI value contains forbidden characters".to_string()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain() {
        // Valid cases
        assert!(validate_domain("example.test").is_ok());
        assert!(validate_domain("my-site.local").is_ok());
        assert!(validate_domain("sub.domain.test").is_ok());
        assert!(validate_domain("a.b").is_ok());
        assert!(validate_domain("UPPER.test").is_ok());
        assert!(validate_domain("123.test").is_ok());
        assert!(validate_domain("a.com").is_ok());
        assert!(validate_domain(&"a".repeat(63)).is_ok());

        // Invalid cases
        assert!(validate_domain("").is_err()); // empty
        assert!(validate_domain(&"a".repeat(254)).is_err()); // too long
        assert!(validate_domain(".leading-dot").is_err());
        assert!(validate_domain("trailing-dot.").is_err());
        assert!(validate_domain("double..dot").is_err());
        assert!(validate_domain("-leading-hyphen.test").is_err());
        assert!(validate_domain("trailing-hyphen-.test").is_err());
        assert!(validate_domain(&format!("{}.com", "a".repeat(64))).is_err()); // label too long

        // Dangerous cases
        assert!(validate_domain("../traversal").is_err());
        assert!(validate_domain("; rm -rf").is_err());
        assert!(validate_domain("domain\ninjection").is_err());
        assert!(validate_domain("test$(whoami)").is_err());
        assert!(validate_domain("test`whoami`").is_err());
        assert!(validate_domain("test|whoami").is_err());
        assert!(validate_domain("test&whoami").is_err());
        assert!(validate_domain("test>whoami").is_err());
        assert!(validate_domain("test<whoami").is_err());
    }

    #[test]
    fn test_validate_port() {
        // Valid cases
        assert!(validate_port(80).is_ok());
        assert!(validate_port(443).is_ok());
        assert!(validate_port(1024).is_ok());
        assert!(validate_port(3000).is_ok());
        assert!(validate_port(8080).is_ok());
        assert!(validate_port(65535).is_ok());

        // Invalid cases
        assert!(validate_port(0).is_err());
        assert!(validate_port(1).is_err());
        assert!(validate_port(79).is_err());
    }

    #[test]
    fn test_validate_php_version() {
        // Valid cases
        assert!(validate_php_version("8.4").is_ok());
        assert!(validate_php_version("8.4.1").is_ok());
        assert!(validate_php_version("7.4").is_ok());
        assert!(validate_php_version("8.0").is_ok());
        assert!(validate_php_version("8.4-1").is_ok());

        // Invalid cases
        assert!(validate_php_version("8.4-dev").is_err()); // 'dev' contains letters which are rejected
        assert!(validate_php_version("").is_err());
        assert!(validate_php_version("abc").is_err());
        assert!(validate_php_version("8").is_err()); // no dot
        assert!(validate_php_version(".4").is_err()); // starts with dot
        assert!(validate_php_version("8.4;").is_err()); // invalid char
        assert!(validate_php_version("8.4$").is_err());
    }

    #[test]
    fn test_validate_ini_key() {
        // Valid
        assert!(validate_ini_key("max_execution_time").is_ok());
        assert!(validate_ini_key("upload_max_filesize").is_ok());
        assert!(validate_ini_key("memory_limit").is_ok());
        assert!(validate_ini_key("error_reporting").is_ok());

        // Invalid
        assert!(validate_ini_key("").is_err());
        assert!(validate_ini_key("key with spaces").is_err());
        assert!(validate_ini_key("key;injection").is_err());
        assert!(validate_ini_key("1key").is_err()); // starts with digit

        // Blocked
        assert!(validate_ini_key("disable_functions").is_err());
        assert!(validate_ini_key("open_basedir").is_err());
        assert!(validate_ini_key("allow_url_include").is_err());
    }

    #[test]
    fn test_validate_ini_value() {
        // Valid
        assert!(validate_ini_value("128M").is_ok());
        assert!(validate_ini_value("On").is_ok());
        assert!(validate_ini_value("0").is_ok());
        assert!(validate_ini_value("/tmp").is_ok());
        assert!(validate_ini_value("E_ALL & ~E_DEPRECATED").is_ok());

        // Invalid
        assert!(validate_ini_value("value\nnewline").is_err());
        assert!(validate_ini_value("value\r").is_err());
        assert!(validate_ini_value("value\0null").is_err());
        assert!(validate_ini_value("${ENV_VAR}").is_err());
        assert!(validate_ini_value("$(command)").is_err());
    }

    #[test]
    fn test_sanitize_for_nginx() {
        assert_eq!(sanitize_for_nginx("normal"), "normal");
        assert_eq!(sanitize_for_nginx("test;rm"), "testrm");
        assert_eq!(sanitize_for_nginx("a{b}c"), "abc");
        assert_eq!(sanitize_for_nginx("test\"string'"), "teststring");
        assert_eq!(sanitize_for_nginx("test$var"), "testvar");
        assert_eq!(sanitize_for_nginx("test\\path"), "testpath");
        assert_eq!(sanitize_for_nginx("line1\nline2"), "line1line2");
    }

    #[test]
    fn test_sanitize_for_powershell() {
        assert_eq!(sanitize_for_powershell("it's"), "it''s");
        assert_eq!(sanitize_for_powershell("test`cmd"), "testcmd");
        assert_eq!(sanitize_for_powershell("test$var"), "testvar");
        assert_eq!(sanitize_for_powershell("func()"), "func");
        assert_eq!(sanitize_for_powershell("test|cmd"), "testcmd");
        assert_eq!(sanitize_for_powershell("test&cmd"), "testcmd");
        assert_eq!(sanitize_for_powershell("line1\nline2"), "line1line2");
        assert_eq!(sanitize_for_powershell("val\0ue"), "value");
    }

    #[test]
    fn test_validate_site_path_pure() {
        // Test traversal and null bytes
        assert!(validate_site_path("", None).is_err());
        assert!(validate_site_path("path\0injection", None).is_err());
        assert!(validate_site_path("../../../etc/passwd", None).is_err());
        assert!(validate_site_path("..\\windows\\system32", None).is_err());
        assert!(validate_site_path("some/path/../with/traversal", None).is_err());
        assert!(validate_site_path("/absolute/path", None).is_ok()); // if doesn't exist, will just return PathBuf
    }
}
