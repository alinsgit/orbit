use scraper::{Html, Selector};
use semver::Version;
use super::types::{ServiceVersion, VersionSource};

fn parse_version(v: &str) -> Option<Version> {
    Version::parse(v).ok()
}

pub async fn fetch_from_apache_lounge() -> Result<Vec<ServiceVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://www.apachelounge.com/download/")
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

    // Find all links that match httpd-X.X.X pattern
    let link_selector = Selector::parse("a[href*='httpd-']").map_err(|_| "Selector error")?;

    let mut versions: Vec<ServiceVersion> = Vec::new();
    // Match patterns like "httpd-2.4.66-260131-Win64-VS18.zip"
    let version_regex =
        regex::Regex::new(r"httpd-(\d+\.\d+\.\d+)-(\d+)-Win64-VS\d+\.zip").unwrap();

    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            if let Some(caps) = version_regex.captures(href) {
                if let (Some(version), Some(_date)) = (caps.get(1), caps.get(2)) {
                    let v = version.as_str().to_string();
                    // Check if we already have this version
                    if versions.iter().any(|sv| sv.version == v) {
                        continue;
                    }

                    // Build full URL
                    let download_url = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https://www.apachelounge.com{}", href)
                    };

                    let filename = href.split('/').last().unwrap_or("apache.zip").to_string();

                    versions.push(ServiceVersion {
                        version: v,
                        download_url,
                        filename,
                        release_date: None,
                        source: VersionSource::Api,
                    });
                }
            }
        }
    }

    // Sort by semver descending
    versions.sort_by(|a, b| {
        let va = parse_version(&a.version);
        let vb = parse_version(&b.version);
        match (va, vb) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            _ => b.version.cmp(&a.version),
        }
    });

    // Take top 5
    versions.truncate(5);

    if versions.is_empty() {
        return Err("No Apache versions found".to_string());
    }

    Ok(versions)
}

pub fn get_fallback() -> Vec<ServiceVersion> {
    vec![
        ServiceVersion {
            version: "2.4.66".to_string(),
            download_url: "https://www.apachelounge.com/download/VS18/binaries/httpd-2.4.66-260131-Win64-VS18.zip".to_string(),
            filename: "httpd-2.4.66-260131-Win64-VS18.zip".to_string(),
            release_date: Some("2026-01-31".to_string()),
            source: VersionSource::Fallback,
        },
    ]
}
