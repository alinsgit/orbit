use tauri::command;
use std::path::PathBuf;
use crate::services::download::download_file;
use tauri::AppHandle;
use tauri::Manager;
use std::fs::File;
use zip::ZipArchive;

/// Extracts a zip file, optionally stripping a common root folder
fn extract_zip(zip_path: &PathBuf, extract_path: &PathBuf, strip_root: bool) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

    // Detect common root folder if strip_root is enabled
    let root_folder = if strip_root && archive.len() > 0 {
        // Check first entry to detect root folder
        if let Ok(first) = archive.by_index(0) {
            let name = first.name();
            // If first entry is a directory (ends with /), use it as root to strip
            if name.ends_with('/') {
                Some(name.to_string())
            } else if let Some(idx) = name.find('/') {
                // If first entry is a file, get the root folder from path
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

    println!("Extracting {} files, root_folder: {:?}", archive.len(), root_folder);

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

        // Skip if the path is empty after stripping (it's the root folder itself)
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let outpath = extract_path.join(&relative_path);

        if (*file.name()).ends_with('/') {
            std::fs::create_dir_all(&outpath).map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(&p).map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
            }
            let mut outfile = File::create(&outpath).map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Failed to write file: {}", e))?;
        }
    }
    Ok(())
}

#[command]
pub async fn download_service(
    app: AppHandle,
    url: String, 
    filename: String,
    service_type: String
) -> Result<String, String> {
    // Base bin path - use app local data dir for portable storage
    let bin_path = app.path().app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");
    
    if !bin_path.exists() {
        std::fs::create_dir_all(&bin_path).map_err(|e| format!("Failed to create bin dir: {}", e))?;
    }

    let downloads_dir = bin_path.join("downloads");
    if !downloads_dir.exists() {
        std::fs::create_dir_all(&downloads_dir).map_err(|e| format!("Failed to create downloads dir: {}", e))?;
    }
    
    let dest_path = downloads_dir.join(&filename);

    println!("Downloading {} from {} to {:?}", service_type, url, dest_path);

    // Download the file
    download_file(&url, &dest_path).await?;

    // Determine extraction target and whether to strip root folder
    let (extract_target, strip_root) = match service_type.as_str() {
        "nginx" => (bin_path.join("nginx"), true),       // nginx-x.x.x/ folder inside
        "mariadb" => (bin_path.join("mariadb"), true),   // mariadb-x.x.x-winx64/ folder inside
        s if s.starts_with("php") => {
            let version = s.strip_prefix("php-").unwrap_or("latest");
            (bin_path.join("php").join(version), false)  // PHP zips have flat structure
        }
        "nodejs" => (bin_path.join("nodejs"), true),     // node-vx.x.x-win-x64/ folder inside
        "python" => (bin_path.join("python"), false),    // Python embed has flat structure
        "bun" => (bin_path.join("bun"), true),           // bun-windows-x64/ folder inside
        "apache" => (bin_path.join("apache"), true),     // Apache24/ folder inside
        _ => (bin_path.join("misc").join(&service_type), false),
    };

    println!("Extracting to {:?} (strip_root: {})", extract_target, strip_root);

    // Clean target directory for fresh install
    if extract_target.exists() {
        std::fs::remove_dir_all(&extract_target)
            .map_err(|e| format!("Failed to clean target dir: {}", e))?;
    }
    std::fs::create_dir_all(&extract_target)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    match extract_zip(&dest_path, &extract_target, strip_root) {
        Ok(_) => {
            // Cleanup zip file after successful extraction
            let _ = std::fs::remove_file(&dest_path);

            // Post-installation setup based on service type
            if service_type.starts_with("php") {
                configure_php(&extract_target)?;
            } else if service_type == "apache" {
                configure_apache(&extract_target)?;
            }

            Ok(format!("Service installed to {:?}", extract_target))
        },
        Err(e) => Err(format!("Extraction failed: {}", e)),
    }
}

/// Move contents from source directory to destination (used for Apache24 subfolder)
fn move_apache_contents(source: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let entries = std::fs::read_dir(source)
        .map_err(|e| format!("Failed to read Apache24 directory: {}", e))?;

    for entry in entries.flatten() {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dest.join(&file_name);

        // Skip if destination already exists
        if dest_path.exists() && dest_path != src_path {
            continue;
        }

        if src_path.is_dir() {
            // Use copy for directories, then remove source
            copy_dir_all(&src_path, &dest_path)?;
            let _ = std::fs::remove_dir_all(&src_path);
        } else {
            std::fs::rename(&src_path, &dest_path)
                .map_err(|e| format!("Failed to move {}: {}", file_name.to_string_lossy(), e))?;
        }
    }

    // Remove empty Apache24 folder
    let _ = std::fs::remove_dir(source);

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create dir {:?}: {}", dst, e))?;

    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {:?}: {}", src_path, e))?;
        }
    }
    Ok(())
}

/// Configure Apache after installation
fn configure_apache(apache_path: &PathBuf) -> Result<(), String> {
    // Apache Lounge zips might have Apache24 subfolder even after stripping
    // Check both direct path and Apache24 subfolder
    let conf_dir = if apache_path.join("conf").exists() {
        apache_path.join("conf")
    } else if apache_path.join("Apache24").join("conf").exists() {
        // Move contents from Apache24 to apache_path
        let apache24_path = apache_path.join("Apache24");
        move_apache_contents(&apache24_path, apache_path)?;
        apache_path.join("conf")
    } else {
        // List directory contents for debugging
        let contents: Vec<_> = std::fs::read_dir(apache_path)
            .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        return Err(format!("httpd.conf not found. Directory contents: {:?}", contents));
    };

    let httpd_conf = conf_dir.join("httpd.conf");

    if !httpd_conf.exists() {
        return Err(format!("httpd.conf not found at {:?}", httpd_conf));
    }

    // Read httpd.conf
    let mut content = std::fs::read_to_string(&httpd_conf)
        .map_err(|e| format!("Failed to read httpd.conf: {}", e))?;

    // Update ServerRoot to use the actual installation path
    let server_root = apache_path.to_string_lossy().replace('\\', "/");

    // Replace the default ServerRoot
    let server_root_regex = regex::Regex::new(r#"(?m)^Define SRVROOT.*$"#).unwrap();
    content = server_root_regex.replace(&content, format!(r#"Define SRVROOT "{}""#, server_root)).to_string();

    // If no SRVROOT define found, try replacing ServerRoot directly
    if !content.contains("SRVROOT") {
        let server_root_regex2 = regex::Regex::new(r#"(?m)^ServerRoot.*$"#).unwrap();
        content = server_root_regex2.replace(&content, format!(r#"ServerRoot "{}""#, server_root)).to_string();
    }

    // Enable common modules
    let modules_to_enable = [
        "mod_rewrite",
        "mod_headers",
        "mod_expires",
        "mod_deflate",
    ];

    for module in modules_to_enable {
        let disabled = format!("#LoadModule {}_module", module);
        let enabled = format!("LoadModule {}_module", module);
        if content.contains(&disabled) {
            content = content.replace(&disabled, &enabled);
        }
    }

    // Set Listen port to 8082 to avoid conflict with nginx (80) and other services
    let listen_regex = regex::Regex::new(r"(?m)^Listen\s+\d+").unwrap();
    content = listen_regex.replace(&content, "Listen 8082").to_string();

    // Update ServerName
    let server_name_regex = regex::Regex::new(r"(?m)^#?ServerName.*$").unwrap();
    content = server_name_regex.replace(&content, "ServerName localhost:8082").to_string();

    // Write updated httpd.conf
    std::fs::write(&httpd_conf, content)
        .map_err(|e| format!("Failed to write httpd.conf: {}", e))?;

    // Create logs directory if it doesn't exist
    let logs_dir = apache_path.join("logs");
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to create logs dir: {}", e))?;
    }

    println!("Apache configured successfully at {:?}", apache_path);
    Ok(())
}

/// Configure PHP after installation
fn configure_php(php_path: &PathBuf) -> Result<(), String> {
    let ini_dev = php_path.join("php.ini-development");
    let ini_prod = php_path.join("php.ini-production");
    let ini_target = php_path.join("php.ini");

    // Copy php.ini-development to php.ini if it doesn't exist
    if !ini_target.exists() {
        if ini_dev.exists() {
            std::fs::copy(&ini_dev, &ini_target)
                .map_err(|e| format!("Failed to create php.ini: {}", e))?;
        } else if ini_prod.exists() {
            std::fs::copy(&ini_prod, &ini_target)
                .map_err(|e| format!("Failed to create php.ini: {}", e))?;
        } else {
            return Err("No php.ini template found".to_string());
        }
    }

    // Read php.ini
    let mut content = std::fs::read_to_string(&ini_target)
        .map_err(|e| format!("Failed to read php.ini: {}", e))?;

    // Set extension_dir
    let ext_dir = php_path.join("ext");
    let ext_dir_str = ext_dir.to_string_lossy().replace('\\', "/");

    // Replace extension_dir setting
    if content.contains(";extension_dir = \"ext\"") {
        content = content.replace(
            ";extension_dir = \"ext\"",
            &format!("extension_dir = \"{}\"", ext_dir_str)
        );
    } else if !content.contains(&format!("extension_dir = \"{}\"", ext_dir_str)) {
        // Add extension_dir if not present
        content = content.replace(
            "[PHP]",
            &format!("[PHP]\nextension_dir = \"{}\"", ext_dir_str)
        );
    }

    // Enable common extensions for Windows
    let extensions = [
        "curl",
        "fileinfo",
        "gd",
        "mbstring",
        "mysqli",
        "openssl",
        "pdo_mysql",
        "zip",
    ];

    for ext in extensions {
        let disabled = format!(";extension={}", ext);
        let enabled = format!("extension={}", ext);
        if content.contains(&disabled) {
            content = content.replace(&disabled, &enabled);
        }
    }

    // Set some development-friendly defaults
    // error_reporting
    if content.contains(";error_reporting = E_ALL") {
        content = content.replace(";error_reporting = E_ALL", "error_reporting = E_ALL");
    }

    // display_errors
    if content.contains("display_errors = Off") {
        content = content.replace("display_errors = Off", "display_errors = On");
    }

    // Write updated php.ini
    std::fs::write(&ini_target, content)
        .map_err(|e| format!("Failed to write php.ini: {}", e))?;

    println!("PHP configured successfully at {:?}", php_path);
    Ok(())
}

#[command]
pub fn check_vc_redist() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = Command::new("reg")
            .args(&[
                "query",
                "HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64",
                "/v",
                "Installed",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to run reg query: {}", e))?;

        if !output.status.success() {
            // It might not be installed, or key doesn't exist
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Look for "Installed    REG_DWORD    0x1"
        if stdout.contains("REG_DWORD") && stdout.contains("0x1") {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(true) // Not needed on non-Windows
    }
}
