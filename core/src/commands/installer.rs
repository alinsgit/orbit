use tauri::command;
use std::path::{Path, PathBuf};
use crate::services::download::{download_file, extract_archive};
use crate::services::hidden_command;
use tauri::AppHandle;
use tauri::Manager;

/// Image names of binaries we ship per service type. Killed before reinstall
/// so an in-flight nginx.exe / php-cgi.exe doesn't keep its install dir as
/// CWD and block `remove_dir_all` / `rename`.
fn image_names_for(service_type: &str) -> &'static [&'static str] {
    match service_type {
        "nginx" => &["nginx.exe"],
        "apache" => &["httpd.exe", "rotatelogs.exe"],
        s if s.starts_with("php") => &["php-cgi.exe", "php.exe"],
        "mariadb" => &["mariadbd.exe", "mysqld.exe", "mariadb.exe", "mysql.exe"],
        "postgresql" => &["postgres.exe", "pg_ctl.exe", "psql.exe"],
        "mongodb" => &["mongod.exe", "mongos.exe", "mongosh.exe"],
        "redis" => &["redis-server.exe", "redis-cli.exe"],
        "nodejs" => &["node.exe"],
        "python" => &["python.exe", "pythonw.exe", "pip.exe"],
        "bun" => &["bun.exe"],
        "go" => &["go.exe", "gofmt.exe"],
        "deno" => &["deno.exe"],
        "mailpit" => &["mailpit.exe"],
        "meilisearch" => &["meilisearch.exe"],
        _ => &[],
    }
}

/// Best-effort: kill any running instance of the service binaries so the
/// install dir can be cleaned. Filters by image name only — narrow to the
/// install target by relying on the fact that Orbit only ever spawns these
/// from its own bin/. Users running unrelated copies of nginx/python/etc.
/// from elsewhere will be impacted; that's the price of letting the user
/// reinstall without manual cleanup.
fn kill_running_binaries(service_type: &str) {
    #[cfg(target_os = "windows")]
    {
        for image in image_names_for(service_type) {
            let _ = hidden_command("taskkill")
                .args(["/F", "/T", "/IM", image])
                .output();
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        for image in image_names_for(service_type) {
            // Strip .exe on Unix for pkill compatibility
            let name = image.trim_end_matches(".exe");
            let _ = std::process::Command::new("pkill")
                .args(["-9", "-f", name])
                .output();
        }
    }
}

/// Download and install a service version.
///
/// `version` is the explicit registry version string (e.g. "1.27.3"). For
/// PHP it's redundant (already encoded in `service_type` like "php-8.4")
/// but accepted for uniformity.
#[command]
pub async fn download_service(
    app: AppHandle,
    url: String,
    filename: String,
    service_type: String,
    version: Option<String>,
) -> Result<String, String> {
    use crate::services::version_manager;

    // Base bin path - use app local data dir for portable storage
    let bin_path = app.path().app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    if !bin_path.exists() {
        std::fs::create_dir_all(&bin_path).map_err(|e| format!("Failed to create bin dir: {e}"))?;
    }

    let downloads_dir = bin_path.join("downloads");
    if !downloads_dir.exists() {
        std::fs::create_dir_all(&downloads_dir).map_err(|e| format!("Failed to create downloads dir: {e}"))?;
    }

    let dest_path = downloads_dir.join(&filename);

    log::info!("Downloading {service_type} from {url} to {dest_path:?}");

    // Download the file
    download_file(&url, &dest_path).await?;

    // Resolve the version we'll record under .versions/<svc>/<ver>/. For
    // PHP the version lives in service_type ("php-8.4"), for everything
    // else we expect the caller to pass it.
    let resolved_version: String = if service_type.starts_with("php") {
        service_type.strip_prefix("php-").unwrap_or("latest").to_string()
    } else {
        match &version {
            Some(v) if !v.is_empty() => v.clone(),
            _ => {
                // Fall back to a deterministic sentinel so we don't lose the
                // install. Users can rename the dir later if needed.
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                format!("unknown-{ts}")
            }
        }
    };

    // Determine extraction target and whether to strip root folder.
    // Versioned services land in `bin/.versions/<svc>/<ver>/` — `bin/<svc>`
    // is a junction that we (re)point at this version after extraction.
    let (extract_target, strip_root) = match service_type.as_str() {
        s if s.starts_with("php") => {
            // PHP keeps the legacy direct layout: `bin/php/<version>/`.
            let target = bin_path.join("php").join(&resolved_version);
            let strip = filename.ends_with(".tar.gz") || filename.ends_with(".tgz");
            (target, strip)
        }
        "rust" => (bin_path.join("rust"), false),
        "mongosh" => (bin_path.join("mongosh_temp"), true),
        other if version_manager::is_versioned(other) => {
            let target = version_manager::version_dir(&bin_path, other, &resolved_version);
            // strip_root semantics preserved per service:
            let strip = !matches!(other, "python" | "deno");
            (target, strip)
        }
        _ => (bin_path.join("misc").join(&service_type), false),
    };

    log::info!("Extracting to {extract_target:?} (strip_root: {strip_root})");

    // Step 1: Kill any running instance of this service's binaries. They
    // hold a handle on (often a CWD reference into) the install dir, which
    // is what makes both remove_dir_all *and* rename fail on Windows.
    if extract_target.exists() {
        let kill_kind = if service_type.starts_with("php") {
            "php"
        } else {
            service_type.as_str()
        };
        kill_running_binaries(kill_kind);
        // Give Windows a moment to release the handles after taskkill.
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    // Step 2: Clean target directory. Strategy in clean_target_dir:
    //   - clear read-only flags (nginx/MariaDB ZIPs preserve them)
    //   - retry remove_dir_all with backoff (AV/Indexer hold handles ~ms)
    //   - fall back to file-by-file delete (skip locked, rename aside)
    //   - last resort: rename the whole directory aside
    if extract_target.exists() {
        clean_target_dir(&extract_target)
            .map_err(|e| format!("Failed to clean target dir: {e}"))?;
    }
    std::fs::create_dir_all(&extract_target)
        .map_err(|e| format!("Failed to create extract dir: {e}"))?;

    // Check if it's a raw executable (like rustup-init) or an archive
    if service_type == "rust" {
        let extension = dest_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let final_binary_name = if extension == "exe" { "rustup-init.exe" } else { "rustup-init" };
        let target_binary_path = extract_target.join(final_binary_name);
        
        match std::fs::copy(&dest_path, &target_binary_path) {
            Ok(_) => {
                let _ = std::fs::remove_file(&dest_path);
                
                // Make executable on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&target_binary_path).unwrap().permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&target_binary_path, perms);
                }
                
                return Ok(format!("Service installed to {extract_target:?}"));
            },
            Err(e) => return Err(format!("Failed to move executable: {e}")),
        }
    }

    match extract_archive(&dest_path, &extract_target, strip_root) {
        Ok(_) => {
            // Cleanup zip file after successful extraction
            let _ = std::fs::remove_file(&dest_path);

            // Post-installation setup based on service type
            if service_type.starts_with("php") {
                configure_php(&extract_target)?;
            } else if service_type == "python" {
                configure_python(&extract_target)?;
            } else if service_type == "apache" {
                configure_apache(&extract_target)?;
            } else if service_type == "mongosh" {
                // mongosh zip extracts to mongosh_temp/bin/mongosh.exe
                // Merge bin/ contents into mongodb/bin/ so find_mongosh_client can locate it
                let mongosh_bin_src = extract_target.join("bin");
                let mongodb_bin = bin_path.join("mongodb").join("bin");
                if mongosh_bin_src.exists() && mongodb_bin.exists() {
                    for entry in std::fs::read_dir(&mongosh_bin_src)
                        .map_err(|e| format!("Failed to read mongosh bin: {e}"))?
                    {
                        let entry = entry.map_err(|e| format!("Failed to read dir entry: {e}"))?;
                        let dest = mongodb_bin.join(entry.file_name());
                        std::fs::copy(entry.path(), &dest)
                            .map_err(|e| format!("Failed to copy mongosh file: {e}"))?;
                    }
                }
                // Clean up temp extraction dir
                let _ = std::fs::remove_dir_all(&extract_target);
                return Ok("MongoDB Shell (mongosh) installed to mongodb/bin/".to_string());
            }
            // Note: PostgreSQL ZIP extracts to postgresql/pgsql/bin/ (nested).
            // Scanner and service.rs handle both flattened and nested structures.

            // Redis: ZIP has double-nested folder (e.g., Redis-8.6.1-Windows-x64-cygwin-with-Service/)
            // After strip_root, one subfolder may remain. Flatten it.
            if service_type == "redis" {
                configure_redis(&extract_target)?;
            }

            // For versioned services, wire up the shared-data layer so
            // user content (nginx conf/sites-enabled, ssl certs, logs)
            // survives version switches. This MUST run before set_active —
            // afterwards the install path is reachable through the junction
            // and we'd be replacing real dirs through a symlinked location,
            // which behaves inconsistently across Windows API surfaces.
            if version_manager::is_versioned(&service_type) {
                match crate::services::shared_data::link_shared_dirs(
                    &bin_path,
                    &service_type,
                    &extract_target,
                ) {
                    Ok(wired) if !wired.is_empty() => {
                        log::info!(
                            "shared_data: wired {} cross-junction(s): {}",
                            wired.len(),
                            wired.join(", ")
                        );
                    }
                    Ok(_) => {} // service has no shared subdirs, nothing to do
                    Err(e) => {
                        // Don't abort the install — user binaries are already
                        // on disk. Surface a warning so the user knows their
                        // configs may be version-local.
                        log::warn!("shared_data wiring failed for {service_type}: {e}");
                    }
                }

                version_manager::set_active(&bin_path, &service_type, &resolved_version)
                    .map_err(|e| format!("Installed but failed to activate junction: {e}"))?;
                log::info!(
                    "Activated {service_type}@{resolved_version} via junction at {}",
                    version_manager::active_link(&bin_path, &service_type).display()
                );
            }

            Ok(format!("Service installed to {extract_target:?}"))
        },
        Err(e) => Err(format!("Extraction failed: {e}")),
    }
}

// ─── Multi-version management commands ──────────────────────────────────

#[derive(serde::Serialize)]
pub struct ServiceVersionList {
    pub service_type: String,
    pub installed: Vec<String>,
    pub active: Option<String>,
}

/// List all installed versions for a service plus the currently active one.
/// Returns an empty `installed` for unknown service types instead of erroring.
#[command]
pub fn list_service_versions(
    app: AppHandle,
    service_type: String,
) -> Result<ServiceVersionList, String> {
    use crate::services::version_manager;

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    if !version_manager::is_versioned(&service_type) {
        // PHP and unknown types: no junction; just enumerate `bin/<svc>/<ver>`.
        // For PHP this is `bin/php/8.4/`, `bin/php/8.5/`, etc.
        let root = bin_path.join(&service_type);
        let mut installed = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        installed.push(name.to_string());
                    }
                }
            }
        }
        installed.sort();
        return Ok(ServiceVersionList {
            service_type,
            installed,
            active: None,
        });
    }

    Ok(ServiceVersionList {
        installed: version_manager::list_versions(&bin_path, &service_type),
        active: version_manager::active_version(&bin_path, &service_type),
        service_type,
    })
}

/// Switch the active version for a service. The `bin/<svc>` junction is
/// repointed at the requested version. Spawn paths pick this up automatically;
/// callers should restart the service to make the new binary take effect.
#[command]
pub fn set_active_service_version(
    app: AppHandle,
    service_type: String,
    version: String,
) -> Result<String, String> {
    use crate::services::version_manager;

    if !version_manager::is_versioned(&service_type) {
        return Err(format!(
            "Service '{service_type}' does not use the multi-version layout"
        ));
    }

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    // Refuse to swap the binary out from under a running process — the user
    // gets a clearer error than an "Access denied" later when something
    // tries to read the junction.
    let kill_kind = service_type.as_str();
    kill_running_binaries(kill_kind);
    std::thread::sleep(std::time::Duration::from_millis(200));

    version_manager::set_active(&bin_path, &service_type, &version)?;
    Ok(format!("Active {service_type} switched to {version}"))
}

/// Remove a single installed version. If it's the active one, the junction
/// is dropped (and re-pointed to any remaining version), then the version
/// dir is deleted.
#[command]
pub fn remove_service_version(
    app: AppHandle,
    service_type: String,
    version: String,
) -> Result<String, String> {
    use crate::services::version_manager;

    let bin_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("bin");

    // Kill in case this version is the running one — same reasoning as
    // download_service.
    kill_running_binaries(&service_type);
    std::thread::sleep(std::time::Duration::from_millis(200));

    if version_manager::is_versioned(&service_type) {
        version_manager::remove_version(&bin_path, &service_type, &version)?;
    } else {
        // PHP and others: each version dir is `bin/<svc>/<ver>/` directly.
        let target = bin_path.join(&service_type).join(&version);
        if target.exists() {
            clean_target_dir(&target)?;
        }
    }

    Ok(format!("Removed {service_type}@{version}"))
}

// ─── Robust target directory cleanup ────────────────────────────────────

/// Robust replacement for `fs::remove_dir_all` on Windows. Layered fallbacks:
///   1. Clear read-only attributes recursively.
///   2. Retry `remove_dir_all` with backoff (transient AV/Indexer handles).
///   3. Try renaming the whole directory aside (works unless the dir itself
///      is held — e.g. it's some process's CWD).
///   4. File-by-file: walk and delete each entry, renaming locked files
///      aside. The directory ends up empty (or near-empty) — extract can
///      then write into it without hitting "directory exists with files".
///
/// Public so `version_manager` can reuse it when removing a single version.
pub fn clean_target_dir(path: &Path) -> Result<(), String> {
    let _ = clear_readonly_recursive(path);

    // (2) remove_dir_all with backoff
    let mut last_err: Option<std::io::Error> = None;
    for delay_ms in [0u64, 150, 400, 1000] {
        if delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
        match std::fs::remove_dir_all(path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                log::warn!("remove_dir_all({}) failed (will retry): {e}", path.display());
                last_err = Some(e);
                let _ = clear_readonly_recursive(path);
            }
        }
    }

    // (3) Rename whole directory aside
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let aside = path.with_file_name(format!(
        "{}.old-{}",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("orbit-stale"),
        ts
    ));
    if std::fs::rename(path, &aside).is_ok() {
        log::warn!(
            "Renamed locked '{}' aside as '{}'.",
            path.display(),
            aside.display()
        );
        let aside_clone = aside.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let _ = std::fs::remove_dir_all(&aside_clone);
        });
        return Ok(());
    }

    // (4) File-by-file: dir itself is held (likely a process CWD). Empty
    //     it as much as possible so extraction can overwrite the rest.
    log::warn!(
        "'{}' is held by a running process — falling back to file-by-file cleanup.",
        path.display()
    );
    match drain_directory_aside(path, ts) {
        Ok(stuck_count) => {
            if stuck_count == 0 {
                Ok(())
            } else {
                log::warn!(
                    "{stuck_count} item(s) in '{}' could not be cleaned; \
                     extraction will overwrite reachable files.",
                    path.display()
                );
                Ok(())
            }
        }
        Err(e) => {
            let original = last_err
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            Err(format!(
                "{original}. The folder is locked even at the file level: {e}. \
                 Stop the running service (Services tab → Stop) or close any \
                 program using files in '{}' and try again.",
                path.display()
            ))
        }
    }
}

/// Walk `path` and try to remove or rename-aside every entry inside it,
/// without removing `path` itself. Returns the count of entries that
/// resisted both delete and rename.
fn drain_directory_aside(path: &Path, ts: u64) -> std::io::Result<usize> {
    let mut stuck = 0usize;
    let entries = std::fs::read_dir(path)?;
    for entry in entries.flatten() {
        let p = entry.path();
        let _ = clear_readonly_recursive(&p);

        // First try the cheap path: delete.
        let delete_result = if p.is_dir() {
            std::fs::remove_dir_all(&p)
        } else {
            std::fs::remove_file(&p)
        };
        if delete_result.is_ok() {
            continue;
        }

        // Fall back to renaming this entry into a sibling .locked-<ts> dir
        // so the original name is free for the upcoming extraction.
        let parking = path.with_file_name(format!(
            "{}.locked-{}",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("orbit-stale"),
            ts
        ));
        let _ = std::fs::create_dir_all(&parking);
        let target = parking.join(entry.file_name());
        if std::fs::rename(&p, &target).is_err() {
            stuck += 1;
            log::warn!("Could not free '{}' — neither delete nor rename worked.", p.display());
        }
    }
    Ok(stuck)
}

/// Walk a directory and clear the read-only attribute from every file/dir.
/// Best-effort: errors are swallowed because this is just preparation for
/// a delete that may itself succeed without it.
fn clear_readonly_recursive(path: &Path) -> std::io::Result<()> {
    let meta = std::fs::metadata(path)?;
    let mut perms = meta.permissions();
    if perms.readonly() {
        perms.set_readonly(false);
        let _ = std::fs::set_permissions(path, perms);
    }
    if meta.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let _ = clear_readonly_recursive(&entry.path());
            }
        }
    }
    Ok(())
}

/// Move contents from source directory to destination (used for Apache24 subfolder)
fn move_subfolder_up(source: &Path, dest: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(source)
        .map_err(|e| format!("Failed to read subfolder: {e}"))?;

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

    // Remove empty subfolder
    let _ = std::fs::remove_dir(source);

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create dir {dst:?}: {e}"))?;

    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {src_path:?}: {e}"))?;
        }
    }
    Ok(())
}

/// Configure Apache after installation
fn configure_apache(apache_path: &Path) -> Result<(), String> {
    // Apache Lounge zips might have Apache24 subfolder even after stripping
    // Check both direct path and Apache24 subfolder
    let conf_dir = if apache_path.join("conf").exists() {
        apache_path.join("conf")
    } else if apache_path.join("Apache24").join("conf").exists() {
        // Move contents from Apache24 to apache_path
        let apache24_path = apache_path.join("Apache24");
        move_subfolder_up(&apache24_path, apache_path)?;
        apache_path.join("conf")
    } else {
        // List directory contents for debugging
        let contents: Vec<_> = std::fs::read_dir(apache_path)
            .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        return Err(format!("httpd.conf not found. Directory contents: {contents:?}"));
    };

    let httpd_conf = conf_dir.join("httpd.conf");

    if !httpd_conf.exists() {
        return Err(format!("httpd.conf not found at {httpd_conf:?}"));
    }

    // Read httpd.conf
    let mut content = std::fs::read_to_string(&httpd_conf)
        .map_err(|e| format!("Failed to read httpd.conf: {e}"))?;

    // Update ServerRoot to use the actual installation path
    let server_root = apache_path.to_string_lossy().replace('\\', "/");

    // Replace the default ServerRoot
    let server_root_regex = regex::Regex::new(r#"(?m)^Define SRVROOT.*$"#).unwrap();
    content = server_root_regex.replace(&content, format!(r#"Define SRVROOT "{server_root}""#)).to_string();

    // If no SRVROOT define found, try replacing ServerRoot directly
    if !content.contains("SRVROOT") {
        let server_root_regex2 = regex::Regex::new(r#"(?m)^ServerRoot.*$"#).unwrap();
        content = server_root_regex2.replace(&content, format!(r#"ServerRoot "{server_root}""#)).to_string();
    }

    // Enable common modules
    let modules_to_enable = [
        "mod_rewrite",
        "mod_headers",
        "mod_expires",
        "mod_deflate",
    ];

    for module in modules_to_enable {
        let disabled = format!("#LoadModule {module}_module");
        let enabled = format!("LoadModule {module}_module");
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
        .map_err(|e| format!("Failed to write httpd.conf: {e}"))?;

    // Create logs directory if it doesn't exist
    let logs_dir = apache_path.join("logs");
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to create logs dir: {e}"))?;
    }

    log::info!("Apache configured successfully at {apache_path:?}");
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
                .map_err(|e| format!("Failed to create php.ini: {e}"))?;
        } else if ini_prod.exists() {
            std::fs::copy(&ini_prod, &ini_target)
                .map_err(|e| format!("Failed to create php.ini: {e}"))?;
        } else {
            return Err("No php.ini template found".to_string());
        }
    }

    // Read php.ini
    let mut content = std::fs::read_to_string(&ini_target)
        .map_err(|e| format!("Failed to read php.ini: {e}"))?;

    // Set extension_dir
    let ext_dir = php_path.join("ext");
    let ext_dir_str = ext_dir.to_string_lossy().replace('\\', "/");

    // Replace extension_dir setting
    if content.contains(";extension_dir = \"ext\"") {
        content = content.replace(
            ";extension_dir = \"ext\"",
            &format!("extension_dir = \"{ext_dir_str}\"")
        );
    } else if !content.contains(&format!("extension_dir = \"{ext_dir_str}\"")) {
        // Add extension_dir if not present
        content = content.replace(
            "[PHP]",
            &format!("[PHP]\nextension_dir = \"{ext_dir_str}\"")
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
        let disabled = format!(";extension={ext}");
        let enabled = format!("extension={ext}");
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
        .map_err(|e| format!("Failed to write php.ini: {e}"))?;

    log::info!("PHP configured successfully at {php_path:?}");
    Ok(())
}

/// Configure Python after installation — enable site-packages and bootstrap pip
fn configure_python(python_path: &Path) -> Result<(), String> {
    use crate::services::hidden_command;

    // Step 1: Find and fix ._pth file (e.g., python314._pth)
    // Embeddable Python has `#import site` commented out — we need it enabled for pip/packages
    if let Ok(entries) = std::fs::read_dir(python_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with("._pth") {
                let pth_path = entry.path();
                if let Ok(content) = std::fs::read_to_string(&pth_path) {
                    if content.contains("#import site") {
                        let fixed = content.replace("#import site", "import site");
                        std::fs::write(&pth_path, fixed)
                            .map_err(|e| format!("Failed to update {name}: {e}"))?;
                        log::info!("Python: enabled import site in {name}");
                    }
                }
                break;
            }
        }
    }

    // Step 2: Download get-pip.py and bootstrap pip
    #[cfg(target_os = "windows")]
    let python_exe = python_path.join("python.exe");
    #[cfg(not(target_os = "windows"))]
    let python_exe = python_path.join("bin").join("python3");

    let get_pip_path = python_path.join("get-pip.py");

    // Download get-pip.py
    let resp = reqwest::blocking::get("https://bootstrap.pypa.io/get-pip.py")
        .map_err(|e| format!("Failed to download get-pip.py: {e}"))?;
    let bytes = resp.bytes().map_err(|e| format!("Failed to read get-pip.py: {e}"))?;
    std::fs::write(&get_pip_path, &bytes)
        .map_err(|e| format!("Failed to write get-pip.py: {e}"))?;

    // Run get-pip.py
    let output = hidden_command(&python_exe)
        .arg(get_pip_path.to_string_lossy().as_ref())
        .output()
        .map_err(|e| format!("Failed to run get-pip.py: {e}"))?;

    // Clean up get-pip.py regardless of result
    let _ = std::fs::remove_file(&get_pip_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("get-pip.py failed (non-fatal): {stderr}");
        // Non-fatal — pip can still be installed manually later
    } else {
        log::info!("Python: pip bootstrapped successfully");
    }

    Ok(())
}

/// Flatten Redis directory if binaries are in a subfolder
fn configure_redis(redis_path: &Path) -> Result<(), String> {
    // If redis-server.exe is directly in redis/, nothing to do
    if redis_path.join("redis-server.exe").exists() {
        return Ok(());
    }

    // Find the subfolder containing redis-server.exe
    let entries: Vec<_> = std::fs::read_dir(redis_path)
        .map_err(|e| format!("Failed to read redis dir: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    for entry in &entries {
        if entry.path().join("redis-server.exe").exists() {
            log::info!("Redis: flattening subfolder {:?}", entry.file_name());
            move_subfolder_up(&entry.path(), redis_path)?;
            return Ok(());
        }
    }

    // List contents for debugging
    let contents: Vec<_> = std::fs::read_dir(redis_path)
        .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    Err(format!("redis-server.exe not found after extraction. Directory contents: {contents:?}"))
}

#[command]
pub fn check_vc_redist() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = Command::new("reg")
            .args([
                "query",
                "HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64",
                "/v",
                "Installed",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to run reg query: {e}"))?;

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
