use serde::Deserialize;
use semver::Version;
use super::types::{ServiceVersion, VersionSource};

#[derive(Debug, Deserialize)]
struct MariaDbRelease {
    #[allow(dead_code)]
    id: String,
    release_number: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct MariaDbApiResponse {
    #[serde(default)]
    releases: Vec<MariaDbRelease>,
}

fn build_download_url(version: &str) -> String {
    format!(
        "https://archive.mariadb.org/mariadb-{}/winx64-packages/mariadb-{}-winx64.zip",
        version, version
    )
}

fn build_filename(version: &str) -> String {
    format!("mariadb-{}-winx64.zip", version)
}

fn parse_version(v: &str) -> Option<Version> {
    Version::parse(v).ok()
}

pub async fn fetch_from_mariadb_api() -> Result<Vec<ServiceVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Orbit/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://downloads.mariadb.org/rest-api/mariadb/all-releases/")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let data: MariaDbApiResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    // Filter stable releases and sort by version
    let mut stable_versions: Vec<(String, Option<String>)> = data
        .releases
        .into_iter()
        .filter(|r| r.status.to_lowercase() == "stable")
        .map(|r| (r.release_number.clone(), None))
        .collect();

    // Sort by semver descending
    stable_versions.sort_by(|a, b| {
        let va = parse_version(&a.0);
        let vb = parse_version(&b.0);
        match (va, vb) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            _ => b.0.cmp(&a.0),
        }
    });

    // Take top 5
    let result: Vec<ServiceVersion> = stable_versions
        .into_iter()
        .take(5)
        .map(|(version, date)| ServiceVersion {
            version: version.clone(),
            download_url: build_download_url(&version),
            filename: build_filename(&version),
            release_date: date,
            source: VersionSource::Api,
        })
        .collect();

    if result.is_empty() {
        return Err("No MariaDB versions found".to_string());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_url() {
        let url = build_download_url("11.4.5");
        assert!(url.contains("mariadb-11.4.5"));
        assert!(url.contains("winx64.zip"));
    }
}
