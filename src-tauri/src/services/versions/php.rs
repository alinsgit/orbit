use serde::Deserialize;
use std::collections::HashMap;
use super::types::{ServiceVersion, VersionSource};

#[derive(Debug, Deserialize)]
struct PhpRelease {
    version: String,
    #[serde(rename = "nts-vs16-x64")]
    nts_vs16_x64: Option<PhpBuild>,
    #[serde(rename = "nts-vs17-x64")]
    nts_vs17_x64: Option<PhpBuild>,
}

#[derive(Debug, Deserialize)]
struct PhpBuild {
    mtime: Option<String>,
    zip: PhpZip,
}

#[derive(Debug, Deserialize)]
struct PhpZip {
    path: String,
    #[allow(dead_code)]
    size: String,
}

pub async fn fetch_from_windows_php() -> Result<Vec<ServiceVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Orbit/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://windows.php.net/downloads/releases/releases.json")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let data: HashMap<String, PhpRelease> = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let mut versions: Vec<ServiceVersion> = data
        .into_iter()
        .filter(|(cycle, _)| {
            // Only PHP 8.x versions
            cycle.starts_with("8.")
        })
        .filter_map(|(_, release)| {
            // Prefer vs17, fallback to vs16
            let build = release.nts_vs17_x64.or(release.nts_vs16_x64)?;

            let release_date = build.mtime.map(|m| {
                // Parse "2025-12-17T12:58:38+01:00" to "2025-12-17"
                m.split('T').next().unwrap_or(&m).to_string()
            });

            Some(ServiceVersion {
                version: release.version.clone(),
                download_url: format!("https://windows.php.net/downloads/releases/{}", build.zip.path),
                filename: build.zip.path,
                release_date,
                source: VersionSource::Api,
            })
        })
        .collect();

    // Sort by version descending
    versions.sort_by(|a, b| {
        let parse_version = |v: &str| -> (u32, u32, u32) {
            let parts: Vec<&str> = v.split('.').collect();
            (
                parts.get(0).and_then(|p| p.parse().ok()).unwrap_or(0),
                parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0),
                parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0),
            )
        };
        parse_version(&b.version).cmp(&parse_version(&a.version))
    });

    // Take top 5
    versions.truncate(5);

    if versions.is_empty() {
        return Err("No PHP versions found".to_string());
    }

    Ok(versions)
}

pub fn get_fallback() -> Vec<ServiceVersion> {
    vec![
        ServiceVersion {
            version: "8.4.16".to_string(),
            download_url: "https://windows.php.net/downloads/releases/php-8.4.16-nts-Win32-vs17-x64.zip".to_string(),
            filename: "php-8.4.16-nts-Win32-vs17-x64.zip".to_string(),
            release_date: Some("2025-12-17".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "8.3.29".to_string(),
            download_url: "https://windows.php.net/downloads/releases/php-8.3.29-nts-Win32-vs16-x64.zip".to_string(),
            filename: "php-8.3.29-nts-Win32-vs16-x64.zip".to_string(),
            release_date: Some("2025-12-17".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "8.2.30".to_string(),
            download_url: "https://windows.php.net/downloads/releases/php-8.2.30-nts-Win32-vs16-x64.zip".to_string(),
            filename: "php-8.2.30-nts-Win32-vs16-x64.zip".to_string(),
            release_date: Some("2025-12-17".to_string()),
            source: VersionSource::Fallback,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_versions() {
        let result = fetch_from_windows_php().await;
        assert!(result.is_ok());
        let versions = result.unwrap();
        assert!(!versions.is_empty());
        // All versions should be 8.x
        for v in &versions {
            assert!(v.version.starts_with("8."));
        }
    }
}
