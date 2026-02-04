use serde::Deserialize;
use super::types::{ServiceVersion, VersionSource};

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub async fn fetch_from_github() -> Result<Vec<ServiceVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Orbit/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://api.github.com/repos/oven-sh/bun/releases")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let releases: Vec<GithubRelease> = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let versions: Vec<ServiceVersion> = releases
        .into_iter()
        .filter(|r| {
            // Skip prereleases (canary, etc)
            !r.tag_name.contains("canary") && !r.tag_name.contains("alpha") && !r.tag_name.contains("beta")
        })
        .take(5)
        .filter_map(|r| {
            // Find Windows x64 asset - exact match to avoid profile/baseline builds
            let windows_asset = r.assets.iter().find(|a| {
                a.name == "bun-windows-x64.zip"
            })?;

            let version = r.tag_name.trim_start_matches("bun-v").trim_start_matches('v').to_string();
            let release_date = r.published_at.map(|d| d.split('T').next().unwrap_or(&d).to_string());

            Some(ServiceVersion {
                version: version.clone(),
                download_url: windows_asset.browser_download_url.clone(),
                filename: windows_asset.name.clone(),
                release_date,
                source: VersionSource::Api,
            })
        })
        .collect();

    if versions.is_empty() {
        return Err("No Bun versions found".to_string());
    }

    Ok(versions)
}

pub fn get_fallback() -> Vec<ServiceVersion> {
    vec![
        ServiceVersion {
            version: "1.2.4".to_string(),
            download_url: "https://github.com/oven-sh/bun/releases/download/bun-v1.2.4/bun-windows-x64.zip".to_string(),
            filename: "bun-windows-x64.zip".to_string(),
            release_date: Some("2025-01-20".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "1.1.45".to_string(),
            download_url: "https://github.com/oven-sh/bun/releases/download/bun-v1.1.45/bun-windows-x64.zip".to_string(),
            filename: "bun-windows-x64.zip".to_string(),
            release_date: Some("2024-12-15".to_string()),
            source: VersionSource::Fallback,
        },
    ]
}
