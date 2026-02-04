use scraper::{Html, Selector};
use semver::Version;
use super::types::{ServiceVersion, VersionSource};

fn build_download_url(version: &str) -> String {
    format!("https://nginx.org/download/nginx-{}.zip", version)
}

fn build_filename(version: &str) -> String {
    format!("nginx-{}.zip", version)
}

fn parse_version(v: &str) -> Option<Version> {
    Version::parse(v).ok()
}

pub async fn fetch_from_nginx_org() -> Result<Vec<ServiceVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Orbit/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://nginx.org/en/download.html")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("Text error: {}", e))?;

    let document = Html::parse_document(&html);

    // Find all links that match nginx-X.X.X pattern
    let link_selector = Selector::parse("a[href*='nginx-']").map_err(|_| "Selector error")?;

    let mut versions: Vec<String> = Vec::new();
    let version_regex = regex::Regex::new(r"nginx-(\d+\.\d+\.\d+)").unwrap();

    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            if let Some(caps) = version_regex.captures(href) {
                if let Some(version) = caps.get(1) {
                    let v = version.as_str().to_string();
                    if !versions.contains(&v) {
                        versions.push(v);
                    }
                }
            }
        }
    }

    // Sort by semver descending
    versions.sort_by(|a, b| {
        let va = parse_version(a);
        let vb = parse_version(b);
        match (va, vb) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            _ => b.cmp(a),
        }
    });

    // Take top 5
    let result: Vec<ServiceVersion> = versions
        .into_iter()
        .take(5)
        .map(|v| ServiceVersion {
            version: v.clone(),
            download_url: build_download_url(&v),
            filename: build_filename(&v),
            release_date: None,
            source: VersionSource::Api,
        })
        .collect();

    if result.is_empty() {
        return Err("No Nginx versions found".to_string());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_url() {
        let url = build_download_url("1.28.1");
        assert_eq!(url, "https://nginx.org/download/nginx-1.28.1.zip");
    }
}
