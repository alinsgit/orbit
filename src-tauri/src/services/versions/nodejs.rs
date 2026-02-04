use serde::Deserialize;
use super::types::{ServiceVersion, VersionSource};

#[derive(Debug, Deserialize)]
struct NodeVersion {
    version: String,
    date: Option<String>,
    lts: serde_json::Value, // can be false or string like "Iron"
}

pub async fn fetch_from_nodejs_org() -> Result<Vec<ServiceVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Orbit/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://nodejs.org/dist/index.json")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let data: Vec<NodeVersion> = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    // Get latest LTS versions and latest current
    let mut versions: Vec<ServiceVersion> = Vec::new();
    let mut seen_lts: std::collections::HashSet<String> = std::collections::HashSet::new();

    for v in data.iter() {
        // Remove 'v' prefix
        let version = v.version.trim_start_matches('v').to_string();

        // Check if LTS
        let is_lts = match &v.lts {
            serde_json::Value::String(s) => Some(s.clone()),
            _ => None,
        };

        if let Some(lts_name) = is_lts {
            // Only add one version per LTS line
            if !seen_lts.contains(&lts_name) && versions.len() < 4 {
                seen_lts.insert(lts_name.clone());
                versions.push(ServiceVersion {
                    version: version.clone(),
                    download_url: format!(
                        "https://nodejs.org/dist/v{}/node-v{}-win-x64.zip",
                        version, version
                    ),
                    filename: format!("node-v{}-win-x64.zip", version),
                    release_date: v.date.clone(),
                    source: VersionSource::Api,
                });
            }
        } else if versions.is_empty() {
            // Add latest current if we haven't added anything yet
            versions.push(ServiceVersion {
                version: version.clone(),
                download_url: format!(
                    "https://nodejs.org/dist/v{}/node-v{}-win-x64.zip",
                    version, version
                ),
                filename: format!("node-v{}-win-x64.zip", version),
                release_date: v.date.clone(),
                source: VersionSource::Api,
            });
        }

        if versions.len() >= 5 {
            break;
        }
    }

    if versions.is_empty() {
        return Err("No Node.js versions found".to_string());
    }

    Ok(versions)
}

pub fn get_fallback() -> Vec<ServiceVersion> {
    vec![
        ServiceVersion {
            version: "22.16.0".to_string(),
            download_url: "https://nodejs.org/dist/v22.16.0/node-v22.16.0-win-x64.zip".to_string(),
            filename: "node-v22.16.0-win-x64.zip".to_string(),
            release_date: Some("2025-01-21".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "20.18.3".to_string(),
            download_url: "https://nodejs.org/dist/v20.18.3/node-v20.18.3-win-x64.zip".to_string(),
            filename: "node-v20.18.3-win-x64.zip".to_string(),
            release_date: Some("2025-01-21".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "18.20.6".to_string(),
            download_url: "https://nodejs.org/dist/v18.20.6/node-v18.20.6-win-x64.zip".to_string(),
            filename: "node-v18.20.6-win-x64.zip".to_string(),
            release_date: Some("2025-01-21".to_string()),
            source: VersionSource::Fallback,
        },
    ]
}
