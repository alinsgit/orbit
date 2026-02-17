use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use futures_util::StreamExt;
use reqwest::Client;

/// Known mirror fallbacks for URLs that serve HTML instead of direct downloads.
/// Returns an alternative URL if one is known, otherwise None.
fn get_mirror_url(url: &str) -> Option<String> {
    // MariaDB: downloads.mariadb.org/f/ serves HTML mirror-picker pages
    // Rewrite to mirror.kumi.systems which serves files directly
    if url.contains("downloads.mariadb.org/f/") {
        // e.g. https://downloads.mariadb.org/f/mariadb-11.4.10/winx64-packages/mariadb-11.4.10-winx64.zip
        //   -> https://mirror.kumi.systems/mariadb/mariadb-11.4.10/winx64-packages/mariadb-11.4.10-winx64.zip
        let after_f = url.split("downloads.mariadb.org/f/").nth(1)?;
        return Some(format!("https://mirror.kumi.systems/mariadb/{}", after_f));
    }
    None
}

pub async fn download_file(url: &str, dest_path: &PathBuf) -> Result<(), String> {
    // Try the primary URL first, then fall back to mirror if HTML is received
    let urls_to_try: Vec<String> = {
        let mut urls = vec![url.to_string()];
        if let Some(mirror) = get_mirror_url(url) {
            urls.push(mirror);
        }
        urls
    };

    let mut last_error = String::new();

    for attempt_url in &urls_to_try {
        match download_file_single(attempt_url, dest_path).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                log::warn!("Download failed for {}: {}", attempt_url, e);
                last_error = e;
            }
        }
    }

    Err(last_error)
}

async fn download_file_single(url: &str, dest_path: &PathBuf) -> Result<(), String> {
    // Create client with proper User-Agent and redirect policy
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(300)) // 5 min timeout for large files
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    // Retry up to 3 times with exponential backoff
    let max_retries = 3;
    let mut last_error = String::new();

    for attempt in 0..max_retries {
        if attempt > 0 {
            let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32));
            log::info!("Retry attempt {} after {:?} delay for: {}", attempt + 1, delay, url);
            tokio::time::sleep(delay).await;
        }

        log::info!("Downloading from: {} (attempt {})", url, attempt + 1);
        
        let res = match client.get(url)
            .header("Accept", "application/octet-stream, application/zip, application/x-gzip, */*;q=0.1")
            .header("Accept-Encoding", "identity") // Don't compress the binary download
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("Failed to send request: {}", e);
                continue;
            }
        };

        if !res.status().is_success() {
            last_error = format!("Download failed with status: {}", res.status());
            continue;
        }

        // Check content type - reject HTML responses (CloudFlare challenge pages)
        if let Some(content_type) = res.headers().get("content-type") {
            let ct = content_type.to_str().unwrap_or("");
            if ct.contains("text/html") {
                last_error = format!(
                    "Download blocked: received HTML instead of file (possibly CloudFlare protection). URL: {}", 
                    url
                );
                // Don't retry for HTML responses - this won't change with retries
                return Err(last_error);
            }
        }

        // Get content length for progress (optional)
        let total_size = res.content_length().unwrap_or(0);
        log::info!("Download size: {} bytes", total_size);

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let mut file = File::create(dest_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut stream = res.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut stream_error = false;

        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => {
                    if let Err(e) = file.write_all(&chunk) {
                        last_error = format!("Error while writing to file: {}", e);
                        stream_error = true;
                        break;
                    }
                    downloaded += chunk.len() as u64;
                }
                Err(e) => {
                    last_error = format!("Error while downloading chunk: {}", e);
                    stream_error = true;
                    break;
                }
            }
        }

        if stream_error {
            // Clean up partial file
            let _ = std::fs::remove_file(dest_path);
            continue;
        }

        // Verify download completed
        if total_size > 0 && downloaded != total_size {
            last_error = format!("Download incomplete: {} of {} bytes", downloaded, total_size);
            let _ = std::fs::remove_file(dest_path);
            continue;
        }

        log::info!("Download complete: {} bytes written to {:?}", downloaded, dest_path);
        return Ok(());
    }

    Err(last_error)
}

/// Extracts a zip file, optionally stripping a common root folder
pub fn extract_zip(zip_path: &PathBuf, extract_path: &PathBuf) -> Result<(), String> {
    extract_zip_with_strip(zip_path, extract_path, true)
}

/// Extracts a zip file with configurable root folder stripping
pub fn extract_zip_with_strip(zip_path: &PathBuf, extract_path: &PathBuf, strip_root: bool) -> Result<(), String> {
    use zip::ZipArchive;

    let file = File::open(zip_path).map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

    // Detect common root folder if strip_root is enabled
    let root_folder = if strip_root && archive.len() > 0 {
        if let Ok(first) = archive.by_index(0) {
            let name = first.name();
            if name.ends_with('/') {
                Some(name.to_string())
            } else if let Some(idx) = name.find('/') {
                Some(format!("{}/", &name[..idx]))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    log::info!("Extracting {} files, root_folder: {:?}", archive.len(), root_folder);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Failed to read zip entry: {}", e))?;

        // Sanitize file path to prevent zip slip
        let file_path = match file.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => continue,
        };

        // Strip root folder if detected
        let relative_path = if let Some(ref root) = root_folder {
            let path_str = file_path.to_string_lossy();
            if path_str.starts_with(root) {
                PathBuf::from(&path_str[root.len()..])
            } else {
                file_path
            }
        } else {
            file_path
        };

        // Skip if the path is empty after stripping
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let outpath = extract_path.join(&relative_path);

        if (*file.name()).ends_with('/') {
            std::fs::create_dir_all(&outpath).map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
            }
            let mut outfile = File::create(&outpath).map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Failed to copy file: {}", e))?;
        }

        // Set permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)).ok();
            }
        }
    }

    Ok(())
}
