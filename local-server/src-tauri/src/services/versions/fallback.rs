use super::types::{ServiceVersion, VersionSource};

#[allow(dead_code)]
pub fn get_php_fallback() -> Vec<ServiceVersion> {
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
        ServiceVersion {
            version: "8.1.34".to_string(),
            download_url: "https://windows.php.net/downloads/releases/php-8.1.34-nts-Win32-vs16-x64.zip".to_string(),
            filename: "php-8.1.34-nts-Win32-vs16-x64.zip".to_string(),
            release_date: Some("2025-12-17".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "8.0.30".to_string(),
            download_url: "https://windows.php.net/downloads/releases/archives/php-8.0.30-nts-Win32-vs16-x64.zip".to_string(),
            filename: "php-8.0.30-nts-Win32-vs16-x64.zip".to_string(),
            release_date: Some("2023-12-01".to_string()),
            source: VersionSource::Fallback,
        },
    ]
}

pub fn get_nginx_fallback() -> Vec<ServiceVersion> {
    vec![
        ServiceVersion {
            version: "1.28.1".to_string(),
            download_url: "https://nginx.org/download/nginx-1.28.1.zip".to_string(),
            filename: "nginx-1.28.1.zip".to_string(),
            release_date: None,
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "1.26.3".to_string(),
            download_url: "https://nginx.org/download/nginx-1.26.3.zip".to_string(),
            filename: "nginx-1.26.3.zip".to_string(),
            release_date: None,
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "1.24.0".to_string(),
            download_url: "https://nginx.org/download/nginx-1.24.0.zip".to_string(),
            filename: "nginx-1.24.0.zip".to_string(),
            release_date: None,
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "1.22.1".to_string(),
            download_url: "https://nginx.org/download/nginx-1.22.1.zip".to_string(),
            filename: "nginx-1.22.1.zip".to_string(),
            release_date: None,
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "1.20.2".to_string(),
            download_url: "https://nginx.org/download/nginx-1.20.2.zip".to_string(),
            filename: "nginx-1.20.2.zip".to_string(),
            release_date: None,
            source: VersionSource::Fallback,
        },
    ]
}

pub fn get_mariadb_fallback() -> Vec<ServiceVersion> {
    vec![
        ServiceVersion {
            version: "11.4.5".to_string(),
            download_url: "https://archive.mariadb.org/mariadb-11.4.5/winx64-packages/mariadb-11.4.5-winx64.zip".to_string(),
            filename: "mariadb-11.4.5-winx64.zip".to_string(),
            release_date: Some("2024-11-11".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "11.2.6".to_string(),
            download_url: "https://archive.mariadb.org/mariadb-11.2.6/winx64-packages/mariadb-11.2.6-winx64.zip".to_string(),
            filename: "mariadb-11.2.6-winx64.zip".to_string(),
            release_date: Some("2024-11-11".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "10.11.10".to_string(),
            download_url: "https://archive.mariadb.org/mariadb-10.11.10/winx64-packages/mariadb-10.11.10-winx64.zip".to_string(),
            filename: "mariadb-10.11.10-winx64.zip".to_string(),
            release_date: Some("2024-11-11".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "10.6.20".to_string(),
            download_url: "https://archive.mariadb.org/mariadb-10.6.20/winx64-packages/mariadb-10.6.20-winx64.zip".to_string(),
            filename: "mariadb-10.6.20-winx64.zip".to_string(),
            release_date: Some("2024-11-11".to_string()),
            source: VersionSource::Fallback,
        },
        ServiceVersion {
            version: "10.5.27".to_string(),
            download_url: "https://archive.mariadb.org/mariadb-10.5.27/winx64-packages/mariadb-10.5.27-winx64.zip".to_string(),
            filename: "mariadb-10.5.27-winx64.zip".to_string(),
            release_date: Some("2024-11-11".to_string()),
            source: VersionSource::Fallback,
        },
    ]
}
