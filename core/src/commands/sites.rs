use crate::services::nginx::NginxManager;
use crate::services::site_process::SiteProcessManager;
use crate::services::site_store::{SiteMetadata, SiteStore};
use crate::services::sites::{Site, SiteManager, SiteWithStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tauri::command;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize)]
pub struct SiteExport {
    pub version: String,
    pub exported_at: String,
    pub sites: Vec<SiteExportEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct SiteExportEntry {
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub php_version: Option<String>,
    pub ssl_enabled: bool,
    pub template: Option<String>,
    pub web_server: String,
}

#[command]
pub fn create_site(app: AppHandle, site: Site) -> Result<SiteWithStatus, String> {
    SiteManager::create_site(&app, site)
}

#[command]
pub fn get_sites(app: AppHandle) -> Result<Vec<SiteWithStatus>, String> {
    SiteManager::get_sites(&app)
}

#[command]
pub fn get_site(app: AppHandle, domain: String) -> Result<Option<SiteWithStatus>, String> {
    SiteManager::get_site(&app, &domain)
}

#[command]
pub fn update_site(app: AppHandle, domain: String, site: Site) -> Result<SiteWithStatus, String> {
    SiteManager::update_site(&app, &domain, site)
}

#[command]
pub fn delete_site(app: AppHandle, domain: String) -> Result<String, String> {
    SiteManager::delete_site(&app, &domain)?;
    Ok("Site deleted successfully".to_string())
}

#[command]
pub fn regenerate_site_config(app: AppHandle, domain: String) -> Result<String, String> {
    SiteManager::regenerate_config(&app, &domain)?;
    Ok("Config regenerated successfully".to_string())
}

#[derive(Serialize, Deserialize)]
pub struct RegenerateAllResult {
    pub regenerated: usize,
    pub failed: usize,
    /// Sites with no local path set (recovery stubs etc.) — counted
    /// separately from `failed` because they need user action, not a fix.
    pub skipped_empty_path: Vec<String>,
    pub errors: Vec<String>,
}

#[command]
pub fn regenerate_all_site_configs(app: AppHandle) -> Result<RegenerateAllResult, String> {
    let store = SiteStore::load(&app)?;
    // Snapshot domain + path so we can pre-filter; otherwise we'd hit
    // regenerate_config with empty paths and rely on its own guard. Doing
    // the filter at this layer also keeps these out of the failure count.
    let entries: Vec<(String, String)> = store
        .sites
        .iter()
        .map(|s| (s.domain.clone(), s.path.clone()))
        .collect();

    let mut regenerated = 0;
    let mut failed = 0;
    let mut skipped_empty_path: Vec<String> = vec![];
    let mut errors: Vec<String> = vec![];

    for (domain, path) in entries {
        if path.trim().is_empty() {
            skipped_empty_path.push(domain);
            continue;
        }
        match SiteManager::regenerate_config(&app, &domain) {
            Ok(_) => regenerated += 1,
            Err(e) => {
                failed += 1;
                errors.push(format!("{domain}: {e}"));
            }
        }
    }

    // Reload nginx & apache once at the end
    let _ = NginxManager::reload(&app);
    let _ = crate::services::apache::ApacheManager::reload(&app);

    skipped_empty_path.sort();
    Ok(RegenerateAllResult {
        regenerated,
        failed,
        skipped_empty_path,
        errors,
    })
}

// Nginx management commands
#[command]
pub fn nginx_test_config(app: AppHandle) -> Result<String, String> {
    NginxManager::test_config(&app)
}

#[command]
pub fn nginx_reload(app: AppHandle) -> Result<String, String> {
    NginxManager::reload(&app)
}

#[command]
pub fn nginx_status() -> Result<bool, String> {
    Ok(NginxManager::is_running())
}

#[command]
pub fn scaffold_basic_project(path: String, template: String) -> Result<String, String> {
    let project_path = Path::new(&path);

    // Create directory
    fs::create_dir_all(project_path)
        .map_err(|e| format!("Failed to create directory: {e}"))?;

    match template.as_str() {
        "http" => {
            let index_php = project_path.join("index.php");
            fs::write(&index_php, "<?php\nphpinfo();\n")
                .map_err(|e| format!("Failed to create index.php: {e}"))?;
            Ok(format!("Created PHP project at {path}"))
        }
        "static" => {
            let index_html = project_path.join("index.html");
            let content = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My Site</title>
</head>
<body>
    <h1>Welcome</h1>
    <p>Your static site is ready.</p>
</body>
</html>"#;
            fs::write(&index_html, content)
                .map_err(|e| format!("Failed to create index.html: {e}"))?;
            Ok(format!("Created static project at {path}"))
        }
        "litecart" => {
            // Just create the directory, user downloads LiteCart manually
            Ok(format!("Created directory at {path}. Download LiteCart files into this folder."))
        }
        _ => Err(format!("Unsupported template for basic scaffold: {template}")),
    }
}

#[command]
pub fn export_sites(app: AppHandle) -> Result<SiteExport, String> {
    let sites = SiteManager::get_sites(&app)?;

    let export_entries: Vec<SiteExportEntry> = sites
        .iter()
        .map(|site| SiteExportEntry {
            name: site.domain.replace(".local", ""),
            domain: site.domain.clone(),
            root_path: site.path.clone(),
            php_version: site.php_version.clone(),
            ssl_enabled: site.ssl_enabled,
            template: site.template.clone(),
            web_server: site.web_server.clone(),
        })
        .collect();

    Ok(SiteExport {
        version: "1.0".to_string(),
        exported_at: chrono::Local::now().to_rfc3339(),
        sites: export_entries,
    })
}

#[command]
pub fn import_sites(app: AppHandle, import_data: SiteExport, skip_existing: bool) -> Result<ImportResult, String> {
    let existing_sites = SiteManager::get_sites(&app)?;
    let existing_domains: Vec<String> = existing_sites.iter().map(|s| s.domain.clone()).collect();

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors: Vec<String> = vec![];

    for entry in import_data.sites {
        // Check if site already exists
        if existing_domains.contains(&entry.domain) {
            if skip_existing {
                skipped += 1;
                continue;
            } else {
                errors.push(format!("Site {} already exists", entry.domain));
                continue;
            }
        }

        // Create site from import entry
        let site = Site {
            domain: entry.domain.clone(),
            path: entry.root_path,
            port: 80,
            php_version: entry.php_version,
            php_port: None,
            ssl_enabled: entry.ssl_enabled,
            template: entry.template,
            web_server: entry.web_server,
            dev_port: None,
            dev_command: None,
            dev_working_dir: None,
        };

        match SiteManager::create_site(&app, site) {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("Failed to import {}: {}", entry.domain, e)),
        }
    }

    Ok(ImportResult {
        imported,
        skipped,
        errors,
    })
}

#[derive(Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

// Per-site nginx config read/write

#[command]
pub fn read_site_config(app: AppHandle, domain: String) -> Result<String, String> {
    let sites_dir = NginxManager::get_sites_dir(&app)?;
    let conf_path = sites_dir.join(format!("{domain}.conf"));

    if !conf_path.exists() {
        return Err(format!("No nginx config found for '{domain}'"));
    }

    fs::read_to_string(&conf_path)
        .map_err(|e| format!("Failed to read config: {e}"))
}

#[command]
pub fn write_site_config(app: AppHandle, domain: String, content: String) -> Result<String, String> {
    let sites_dir = NginxManager::get_sites_dir(&app)?;
    let conf_path = sites_dir.join(format!("{domain}.conf"));

    if !conf_path.exists() {
        return Err(format!("No nginx config found for '{domain}'"));
    }

    // Backup old config
    let backup = fs::read_to_string(&conf_path).ok();

    // Write new config
    fs::write(&conf_path, &content)
        .map_err(|e| format!("Failed to write config: {e}"))?;

    // Validate with nginx -t
    match NginxManager::test_config(&app) {
        Ok(_) => {
            // Reload nginx if running
            if NginxManager::is_running() {
                let _ = NginxManager::reload(&app);
            }
            Ok("Config saved and validated successfully".to_string())
        }
        Err(e) => {
            // Rollback on validation failure
            if let Some(old_content) = backup {
                let _ = fs::write(&conf_path, old_content);
            }
            Err(format!("Config validation failed (rolled back): {e}"))
        }
    }
}

// Site app process management commands

/// Per-domain log path. Sanitized to keep filesystem-unfriendly characters
/// out of file names (windows is strict about :, /, etc.).
fn site_app_log_path(app: &AppHandle, domain: &str) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("logs")
        .join("site-apps");
    let safe: String = domain
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(dir.join(format!("{safe}.log")))
}

#[command]
pub fn start_site_app(
    app: AppHandle,
    state: tauri::State<SiteProcessManager>,
    domain: String,
) -> Result<u32, String> {
    let store = SiteStore::load(&app)?;
    let site = store
        .get_site(&domain)
        .ok_or_else(|| format!("Site {domain} not found"))?;

    let dev_command = site
        .dev_command
        .as_ref()
        .ok_or_else(|| format!("Site {domain} has no dev_command configured"))?;

    let log_path = site_app_log_path(&app, &domain)?;
    // Prefer the explicit dev_working_dir when set — handles Laravel-style
    // sites where path is the doc-root but `php artisan serve` must run
    // from the project parent. Falls back to `path` for the common case
    // (reverse-proxy templates already point path at the project root).
    let working_dir = site
        .dev_working_dir
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&site.path);
    state.start(
        &domain,
        dev_command,
        working_dir,
        site.dev_port,
        Some(&log_path),
    )
}

#[command]
pub fn stop_site_app(
    state: tauri::State<SiteProcessManager>,
    domain: String,
) -> Result<(), String> {
    state.stop(&domain)
}

/// Read the tail of a site app's captured log. `max_bytes` (default 64 KiB)
/// caps how much is sent to the UI so a runaway dev server can't blow up
/// the IPC channel. Returns `None` if the log file doesn't exist yet (the
/// app has never been started).
#[derive(Serialize, Deserialize)]
pub struct SiteAppLog {
    pub path: String,
    pub content: Option<String>,
    pub size_bytes: u64,
    pub truncated: bool,
}

#[command]
pub fn read_site_app_log(
    app: AppHandle,
    domain: String,
    max_bytes: Option<u64>,
) -> Result<SiteAppLog, String> {
    let path = site_app_log_path(&app, &domain)?;
    let path_str = path.to_string_lossy().to_string();

    if !path.exists() {
        return Ok(SiteAppLog {
            path: path_str,
            content: None,
            size_bytes: 0,
            truncated: false,
        });
    }

    let metadata = fs::metadata(&path).map_err(|e| format!("stat({path_str}): {e}"))?;
    let size = metadata.len();
    let limit = max_bytes.unwrap_or(64 * 1024);

    use std::io::{Read, Seek, SeekFrom};
    let mut file = fs::File::open(&path).map_err(|e| format!("open({path_str}): {e}"))?;
    let truncated = size > limit;
    if truncated {
        file.seek(SeekFrom::Start(size - limit))
            .map_err(|e| format!("seek({path_str}): {e}"))?;
    }
    let mut buf = Vec::with_capacity(limit.min(size) as usize);
    file.read_to_end(&mut buf)
        .map_err(|e| format!("read({path_str}): {e}"))?;

    // Lossy is fine — log can carry partial UTF-8 sequences when truncated.
    let content = String::from_utf8_lossy(&buf).to_string();
    Ok(SiteAppLog {
        path: path_str,
        content: Some(content),
        size_bytes: size,
        truncated,
    })
}

#[command]
pub fn get_site_app_status(
    state: tauri::State<SiteProcessManager>,
    domain: String,
) -> Result<String, String> {
    Ok(state.status(&domain))
}

// ─── Site recovery (post-incident) ──────────────────────────────────
//
// Background: a regression in `version_manager::migrate_legacy` deleted the
// user's `bin/nginx/conf/sites-enabled/*.conf` files when migrating an old
// flat install whose target version dir already existed. The deploy-targets
// store, which holds domain → remote_path mappings, was unaffected. This
// command rebuilds `sites.json` stubs from the surviving deploy targets so
// the user only needs to fill in the local path per site.

#[derive(Serialize, Deserialize)]
pub struct RecoverableSite {
    pub domain: String,
    /// Already exists in sites.json — recovery would skip this one.
    pub already_present: bool,
    /// Remote deploy targets attached to this domain (just for display).
    pub deploy_connections: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RecoveryReport {
    pub recovered: Vec<String>,
    pub skipped_existing: Vec<String>,
}

fn read_deploy_target_domains(app: &AppHandle) -> Result<HashMap<String, Vec<String>>, String> {
    let targets_path = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("config")
        .join("deploy-targets.json");
    if !targets_path.exists() {
        return Ok(HashMap::new());
    }
    let raw = fs::read_to_string(&targets_path)
        .map_err(|e| format!("Failed to read deploy-targets.json: {e}"))?;
    // The file is `{ "domain": [{ "connection": "...", "remote_path": "..." }, ...] }`.
    let parsed: HashMap<String, Vec<serde_json::Value>> =
        serde_json::from_str(&raw).map_err(|e| format!("Malformed deploy-targets.json: {e}"))?;
    let mut out = HashMap::new();
    for (domain, targets) in parsed {
        let conns: Vec<String> = targets
            .iter()
            .filter_map(|t| t.get("connection").and_then(|v| v.as_str()).map(String::from))
            .collect();
        out.insert(domain, conns);
    }
    Ok(out)
}

/// Inspect what could be recovered. Returns one entry per domain that
/// appears in `deploy-targets.json`, flagging which already exist in the
/// site store so the UI can show "X of Y can be recovered".
#[command]
pub fn list_recoverable_sites(app: AppHandle) -> Result<Vec<RecoverableSite>, String> {
    let deploy_domains = read_deploy_target_domains(&app)?;
    if deploy_domains.is_empty() {
        return Ok(Vec::new());
    }
    let store = SiteStore::load(&app)?;
    let existing: std::collections::HashSet<String> =
        store.sites.iter().map(|s| s.domain.clone()).collect();

    let mut out: Vec<RecoverableSite> = deploy_domains
        .into_iter()
        .map(|(domain, conns)| RecoverableSite {
            already_present: existing.contains(&domain),
            domain,
            deploy_connections: conns,
        })
        .collect();
    out.sort_by(|a, b| a.domain.cmp(&b.domain));
    Ok(out)
}

/// Try to find the project root for a domain by scanning the configured
/// workspace dir for a folder whose name matches the domain or its prefix.
/// Returns the first existing dir; None when nothing reasonable is found.
fn guess_local_path_for_domain(workspace: &std::path::Path, domain: &str) -> Option<String> {
    if !workspace.is_dir() {
        return None;
    }
    // Domain → candidate folder names. We try, in order:
    //   1. exact match ("foo.local")
    //   2. domain prefix before first dot ("foo.local" → "foo")
    //   3. dot-replaced ("foo.local" → "foo-local")
    //   4. dash variants for multi-segment domains ("foo.bar.lokal" → "foo-bar")
    let prefix = domain.split('.').next().unwrap_or(domain).to_string();
    let dotless = domain.replace('.', "-");
    let candidates = [
        domain.to_string(),
        prefix.clone(),
        dotless.clone(),
        domain.replace('.', "_"),
    ];

    for cand in candidates.iter() {
        let p = workspace.join(cand);
        if p.is_dir() {
            return Some(p.to_string_lossy().replace('\\', "/"));
        }
    }
    None
}

/// Read the workspace path from settings.json. Errors fall through to None
/// so recovery still works without auto-suggest.
fn workspace_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    let settings_path = app
        .path()
        .app_local_data_dir()
        .ok()?
        .join(".settings.json");
    let raw = fs::read_to_string(settings_path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    v.get("workspace_path")
        .or_else(|| v.get("workspace_dir"))
        .or_else(|| v.get("general").and_then(|g| g.get("workspace_path")))
        .and_then(|s| s.as_str())
        .map(std::path::PathBuf::from)
}

/// Write stub Site entries into `sites.json` for every domain in
/// `deploy-targets.json` that's missing from the store. When the workspace
/// path is configured and a same-named folder exists there, the local
/// `path` is filled in automatically — the user just clicks Save. Domains
/// without a guessable folder still get a stub with empty path; the edit
/// flow handles those.
#[command]
pub fn recover_sites_from_deploy_targets(app: AppHandle) -> Result<RecoveryReport, String> {
    let deploy_domains = read_deploy_target_domains(&app)?;
    let mut store = SiteStore::load(&app)?;
    let existing: std::collections::HashSet<String> =
        store.sites.iter().map(|s| s.domain.clone()).collect();

    let workspace = workspace_path(&app);

    let mut recovered = Vec::new();
    let mut skipped_existing = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for (domain, _conns) in deploy_domains {
        if existing.contains(&domain) {
            skipped_existing.push(domain);
            continue;
        }
        let guessed_path = workspace
            .as_deref()
            .and_then(|w| guess_local_path_for_domain(w, &domain))
            .unwrap_or_default();
        store.add_site(SiteMetadata {
            domain: domain.clone(),
            path: guessed_path,
            port: 80,
            php_version: None,
            php_port: None,
            ssl_enabled: false,
            ssl_cert_path: None,
            ssl_key_path: None,
            template: Some("http".to_string()),
            web_server: "nginx".to_string(),
            dev_port: None,
            dev_command: None,
            dev_working_dir: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        });
        recovered.push(domain);
    }

    if !recovered.is_empty() {
        store.save(&app)?;
    }
    recovered.sort();
    skipped_existing.sort();
    Ok(RecoveryReport {
        recovered,
        skipped_existing,
    })
}
