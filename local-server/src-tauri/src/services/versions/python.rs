use serde::Deserialize;
use super::types::{ServiceVersion, VersionSource};

#[derive(Debug, Deserialize)]
struct PythonRelease {
    cycle: String,
    latest: String,
    #[serde(rename = "latestReleaseDate")]
    latest_release_date: Option<String>,
    eol: Option<String>,
}

pub async fn fetch_from_endoflife() -> Result<Vec<ServiceVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Orbit/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://endoflife.date/api/python.json")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let data: Vec<PythonRelease> = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let versions: Vec<ServiceVersion> = data
        .into_iter()
        .filter(|v| {
            // Only Python 3.x versions that haven't reached EOL
            if !v.cycle.starts_with("3.") {
                return false;
            }
            // Check EOL date if available
            if let Some(eol) = &v.eol {
                if eol.as_str() < "2025" {
                    return false;
                }
            }
            true
        })
        .take(5)
        .map(|v| {
            let version = v.latest.clone();
            ServiceVersion {
                version: version.clone(),
                download_url: format!(
                    "https://www.python.org/ftp/python/{}/python-{}-embed-amd64.zip",
                    version, version
                ),
                filename: format!("python-{}-embed-amd64.zip", version),
                release_date: v.latest_release_date,
                source: VersionSource::Api,
            }
        })
        .collect();

    if versions.is_empty() {
        return Err("No Python versions found".to_string());
    }

    Ok(versions)
}

pub fn get_fallback() -> Vec<ServiceVersion> {
    vec![
        ServiceVersion {
            version: "3.13.2".to_string(),
            download_url: "https://www.python.org/ftp/python/3.13.2/python-3.13.2-embed-amd64.zip".to_string(),
            filename: "python-3.13.2-embed-amd64.zip".to_string(),
            release_date: Some("2025-01-01".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "3.12.9".to_string(),
            download_url: "https://www.python.org/ftp/python/3.12.9/python-3.12.9-embed-amd64.zip".to_string(),
            filename: "python-3.12.9-embed-amd64.zip".to_string(),
            release_date: Some("2025-01-01".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "3.11.11".to_string(),
            download_url: "https://www.python.org/ftp/python/3.11.11/python-3.11.11-embed-amd64.zip".to_string(),
            filename: "python-3.11.11-embed-amd64.zip".to_string(),
            release_date: Some("2024-12-01".to_string()),
            source: VersionSource::Fallback,
        },
    ]
}
