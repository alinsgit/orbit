//! Multi-version installation layout (scoop-style "current" junction).
//!
//! ```text
//! bin/
//! ├── .versions/
//! │   ├── nginx/1.27.3/        ← real files
//! │   ├── nginx/1.28.1/
//! │   └── mariadb/11.5.2/
//! ├── nginx/                    ← Windows junction → .versions/nginx/<active>
//! └── mariadb/                  ← Windows junction → .versions/mariadb/<active>
//! ```
//!
//! Spawn paths like `bin/nginx/nginx.exe` keep working unchanged because the
//! junction is transparent. Switching versions only re-points the junction.
//!
//! PHP is intentionally NOT versioned this way: Orbit runs each PHP-FPM on
//! its own port concurrently (`bin/php/8.3/`, `bin/php/8.4/` directly) — the
//! single-active model doesn't apply.

use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use crate::services::hidden_command;

/// Services that follow the multi-version layout. Add a new service here
/// when its install dir should support `bin/.versions/<svc>/<ver>/` + junction.
pub const VERSIONED_SERVICES: &[&str] = &[
    "nginx",
    "apache",
    "mariadb",
    "postgresql",
    "mongodb",
    "redis",
    "nodejs",
    "python",
    "bun",
    "go",
    "deno",
    "mailpit",
    "meilisearch",
];

pub fn is_versioned(service_type: &str) -> bool {
    VERSIONED_SERVICES.contains(&service_type)
}

pub fn versions_root(bin_path: &Path) -> PathBuf {
    bin_path.join(".versions")
}

pub fn version_dir(bin_path: &Path, service_type: &str, version: &str) -> PathBuf {
    versions_root(bin_path).join(service_type).join(version)
}

/// The user-facing path that spawn code already uses (e.g. `bin/nginx/`).
/// After migration this is a junction pointing at the active version dir.
pub fn active_link(bin_path: &Path, service_type: &str) -> PathBuf {
    bin_path.join(service_type)
}

/// List installed versions of a service, sorted lexicographically.
#[allow(dead_code)] // Consumed by upcoming UI version-picker command.
pub fn list_versions(bin_path: &Path, service_type: &str) -> Vec<String> {
    let root = versions_root(bin_path).join(service_type);
    let mut out: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    out
}

/// Returns the version string `bin/<svc>` (the junction) is currently
/// pointing at, or `None` if no junction exists.
#[allow(dead_code)] // Used by remove_version + upcoming UI command.
pub fn active_version(bin_path: &Path, service_type: &str) -> Option<String> {
    let link = active_link(bin_path, service_type);
    let target = std::fs::read_link(&link).ok()?;
    target
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
}

/// Returns true if `path` is a Windows junction (or a Unix symlink). Used to
/// distinguish "we already migrated this" from "this is a flat install dir
/// with real files inside".
pub fn is_junction(path: &Path) -> bool {
    // read_link succeeds for both symlinks and junctions on Windows.
    std::fs::read_link(path).is_ok()
}

/// Replace the `bin/<svc>` junction so it points at a specific version dir.
/// Creates the junction if it doesn't exist.
pub fn set_active(bin_path: &Path, service_type: &str, version: &str) -> Result<(), String> {
    let target = version_dir(bin_path, service_type, version);
    if !target.is_dir() {
        return Err(format!(
            "Version directory does not exist: {}",
            target.display()
        ));
    }

    let link = active_link(bin_path, service_type);

    // Remove existing junction or directory at the link path. We treat a
    // junction as removable with `remove_dir` (it doesn't recurse into the
    // target). A real directory we leave alone — the caller should have
    // already migrated it to .versions/, otherwise we'd lose data.
    if link.exists() {
        if is_junction(&link) {
            std::fs::remove_dir(&link)
                .map_err(|e| format!("Failed to remove existing junction: {e}"))?;
        } else {
            return Err(format!(
                "Refusing to overwrite real directory at '{}'. Migrate it to \
                 '{}' first.",
                link.display(),
                versions_root(bin_path).display()
            ));
        }
    }

    create_junction(&link, &target)
}

#[cfg(target_os = "windows")]
fn create_junction(link: &Path, target: &Path) -> Result<(), String> {
    // mklink /J creates a junction (no admin / dev-mode required, unlike /D).
    // Quote both paths via cmd's /C invocation to handle spaces.
    let output = hidden_command("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(link)
        .arg(target)
        .output()
        .map_err(|e| format!("Failed to invoke mklink: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "mklink /J '{}' '{}' failed: {}",
            link.display(),
            target.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn create_junction(link: &Path, target: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link)
        .map_err(|e| format!("Failed to symlink '{}' -> '{}': {e}", link.display(), target.display()))
}

/// Remove a single installed version. If it's the active one, drop the
/// junction (or repoint to any remaining version).
#[allow(dead_code)] // Wired up by the upcoming uninstall-version Tauri command.
pub fn remove_version(
    bin_path: &Path,
    service_type: &str,
    version: &str,
) -> Result<(), String> {
    let target = version_dir(bin_path, service_type, version);
    let was_active = active_version(bin_path, service_type).as_deref() == Some(version);

    if was_active {
        let link = active_link(bin_path, service_type);
        let _ = std::fs::remove_dir(&link);
    }

    if target.exists() {
        crate::commands::installer::clean_target_dir(&target)
            .map_err(|e| format!("Failed to remove version dir: {e}"))?;
    }

    if was_active {
        // Re-point at any remaining version so spawn paths keep working.
        if let Some(v) = list_versions(bin_path, service_type).first() {
            set_active(bin_path, service_type, v)?;
        }
    }

    Ok(())
}

/// One-shot migration: detect flat installs at `bin/<svc>/` and move them
/// into `bin/.versions/<svc>/<detected-version>/`, then create the junction.
/// Returns the list of "<svc>@<version>" entries that were migrated.
///
/// Skips a service if:
///   - `bin/<svc>` is already a junction (already migrated),
///   - `bin/<svc>` doesn't exist (not installed yet),
///   - we can't reliably detect a version (we don't move data we can't tag).
pub fn migrate_legacy(bin_path: &Path) -> Vec<String> {
    let mut migrated = Vec::new();

    for &service in VERSIONED_SERVICES {
        let link = active_link(bin_path, service);
        if !link.exists() {
            continue;
        }
        if is_junction(&link) {
            continue;
        }

        // Probe the legacy flat install to find the executable + version.
        let (exe_path, version) = match detect_legacy_install(&link, service) {
            Some(v) => v,
            None => {
                log::warn!(
                    "Skipping migration of '{}': could not detect version (no recognized exe at root or one-level deep).",
                    link.display()
                );
                continue;
            }
        };
        log::info!(
            "Migrating legacy install: {} (version {version}, exe at {})",
            link.display(),
            exe_path.display()
        );

        let dest = version_dir(bin_path, service, &version);
        if dest.exists() {
            // Same version was already migrated previously by some other means
            // (or there's a leftover folder). Don't overwrite it; just promote
            // the existing one to active and remove the legacy dir.
            log::info!(
                "Existing version dir '{}' found; rebuilding junction without re-moving.",
                dest.display()
            );
            // Best-effort: drop the legacy dir if it still exists.
            let _ = crate::commands::installer::clean_target_dir(&link);
        } else {
            if let Some(parent) = dest.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    log::error!("Failed to create versions parent for {service}: {e}");
                    continue;
                }
            }
            // Atomic rename — if the user's AV holds a handle this can fail,
            // in which case we leave the legacy install in place untouched.
            if let Err(e) = std::fs::rename(&link, &dest) {
                log::error!(
                    "Failed to move '{}' -> '{}': {e} — leaving legacy install in place.",
                    link.display(),
                    dest.display()
                );
                continue;
            }
        }

        if let Err(e) = set_active(bin_path, service, &version) {
            log::error!(
                "Migration moved '{}' to '{}' but junction creation failed: {e}",
                link.display(),
                dest.display()
            );
            continue;
        }

        migrated.push(format!("{service}@{version}"));
    }

    migrated
}

/// Locate the service's executable somewhere under `root` (root, root/bin/,
/// or one level of subdirs) and return the resolved exe path + parsed
/// version. Returns None when nothing recognizable is found.
fn detect_legacy_install(root: &Path, service: &str) -> Option<(PathBuf, String)> {
    use crate::commands::scanner;

    let exe_name = primary_exe(service)?;

    // Same probe order as the scanner: <root>/<exe>, <root>/bin/<exe>,
    // <root>/<sub>/<exe>, <root>/<sub>/bin/<exe>.
    let mut candidates: Vec<PathBuf> = vec![root.join(exe_name), root.join("bin").join(exe_name)];

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            candidates.push(entry.path().join(exe_name));
            candidates.push(entry.path().join("bin").join(exe_name));
        }
    }

    let exe = candidates.into_iter().find(|p| p.exists())?;
    let version = scanner::probe_version(service, &exe).ok()?;
    Some((exe, version))
}

/// Image name we use to identify a service's primary executable. Kept in
/// sync with what `scanner.rs` looks for.
fn primary_exe(service: &str) -> Option<&'static str> {
    Some(match service {
        "nginx" => "nginx.exe",
        "apache" => "httpd.exe",
        "mariadb" => "mariadbd.exe",
        "postgresql" => "postgres.exe",
        "mongodb" => "mongod.exe",
        "redis" => "redis-server.exe",
        "nodejs" => "node.exe",
        "python" => "python.exe",
        "bun" => "bun.exe",
        "go" => "go.exe",
        "deno" => "deno.exe",
        "mailpit" => "mailpit.exe",
        "meilisearch" => "meilisearch.exe",
        _ => return None,
    })
}
