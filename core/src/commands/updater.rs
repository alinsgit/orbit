use serde::{Deserialize, Serialize};
use tauri::command;

const GITHUB_REPO: &str = "alinsgit/orbit";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: String,
    pub download_url: String,
    pub published_at: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    published_at: Option<String>,
}

/// Compare two semver version strings (e.g. "0.1.5" vs "0.1.6")
/// Returns true if `latest` is newer than `current`
fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let current_parts = parse(current);
    let latest_parts = parse(latest);

    for i in 0..3 {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let l = latest_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

#[command]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    let url = format!(
        "https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
    );

    let client = reqwest::Client::builder()
        .user_agent("Orbit-Desktop-App")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub API returned status: {}",
            response.status()
        ));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {e}"))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let update_available = is_newer_version(CURRENT_VERSION, &latest_version);

    Ok(UpdateInfo {
        update_available,
        current_version: CURRENT_VERSION.to_string(),
        latest_version,
        release_notes: release.body.unwrap_or_default(),
        download_url: release.html_url,
        published_at: release.published_at.unwrap_or_default(),
    })
}

#[command]
pub fn get_current_version() -> String {
    CURRENT_VERSION.to_string()
}
