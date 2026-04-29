//! Shared, version-independent data dirs for services that keep config /
//! logs / certs / user content under their install root.
//!
//! Without this layer, switching versions silently hides user data:
//! `bin/<svc>/conf/sites-enabled/*.conf` lives inside the active version's
//! folder, so when the `bin/<svc>` junction repoints at a freshly installed
//! version with empty defaults, the user's site configs appear to vanish.
//!
//! Layout:
//! ```text
//! bin/
//! ├── .versions/nginx/1.28.1/
//! │   ├── conf/         ← junction → ../../../.shared/nginx/conf/
//! │   ├── logs/         ← junction → ../../../.shared/nginx/logs/
//! │   ├── ssl/          ← junction → ../../../.shared/nginx/ssl/
//! │   └── nginx.exe     (binary, version-specific)
//! ├── .shared/
//! │   └── nginx/
//! │       ├── conf/sites-enabled/foo.conf
//! │       ├── logs/access.log
//! │       └── ssl/cert.pem
//! └── nginx/            ← junction → .versions/nginx/1.28.1/
//! ```
//!
//! The cross-junction is transparent to nginx.exe / httpd.exe — they read
//! and write through it as if the conf dir were a regular subfolder.

use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use crate::services::hidden_command;

/// Subdirectories of an install that hold user/config/log data and must be
/// shared across versions. Returning `&[]` here means "this service keeps
/// nothing under its install dir" — usually because data is already stored
/// in a parallel dir like `<app_data>/data/<svc>/` (e.g. mariadb, postgres,
/// mongodb).
pub fn shared_subdirs(service_type: &str) -> &'static [&'static str] {
    match service_type {
        "nginx" => &["conf", "logs", "ssl", "html"],
        "apache" => &["conf", "logs"],
        // Redis stores redis.conf and redis.log AT THE ROOT of its install
        // dir, not in a subfolder. We don't junction loose files here —
        // those are recreated by ensure-config logic on each install, which
        // is acceptable since redis config is small and regenerated.
        // mariadb / postgresql / mongodb keep data in <app_data>/data/<svc>.
        _ => &[],
    }
}

/// `bin/.shared/<svc>/<sub>/` — the canonical place for shared data.
pub fn shared_dir(bin_path: &Path, service_type: &str, sub: &str) -> PathBuf {
    bin_path.join(".shared").join(service_type).join(sub)
}

/// Wire up cross-junctions for one version dir. Idempotent and conservative:
///   - If the target sub already exists in `.shared/`, it is kept as-is.
///   - If the install's sub has user content but `.shared` is empty, the
///     content is COPIED to `.shared` (not moved — the install copy stays
///     until we replace the dir with a junction in the same call).
///   - The install's sub is then renamed aside (`<sub>.bak-<ts>`) and a
///     junction `<install>/<sub>` → `<.shared>/<sub>` takes its place.
///   - The `.bak-<ts>` is left on disk so the user can recover if needed.
///     Future installs will accumulate one bak per call; not pretty, but
///     non-destructive is the priority.
///
/// Returns the list of "<service>/<sub>" entries that were wired, for logs.
pub fn link_shared_dirs(
    bin_path: &Path,
    service_type: &str,
    install_dir: &Path,
) -> Result<Vec<String>, String> {
    let subs = shared_subdirs(service_type);
    if subs.is_empty() {
        return Ok(Vec::new());
    }

    let mut wired = Vec::new();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    for sub in subs {
        let install_sub = install_dir.join(sub);
        let shared_sub = shared_dir(bin_path, service_type, sub);

        // Bootstrap shared dir (idempotent).
        if !shared_sub.exists() {
            std::fs::create_dir_all(&shared_sub)
                .map_err(|e| format!("Failed to create '{}': {e}", shared_sub.display()))?;
        }

        // Already a junction? Then it was wired earlier — recreate it
        // to be safe (extraction may have replaced it with a real dir).
        if is_junction(&install_sub) {
            // Tear down and re-link in case the target moved.
            let _ = std::fs::remove_dir(&install_sub);
        }

        if install_sub.exists() {
            // Real directory left over from extraction. Merge anything in it
            // INTO shared first (only adding files shared doesn't already
            // have — never overwrite user data), then move the install dir
            // aside as a timestamped bak. This is the safety net for the
            // case where user-authored content (nginx sites-enabled/*.conf,
            // ssl certs, custom html) lives in the install dir from a
            // previous version of Orbit and we're seeing it for the first
            // time in shared.
            if directory_has_files(&install_sub)? {
                merge_into_shared(&install_sub, &shared_sub).map_err(|e| {
                    format!(
                        "Failed to merge '{}' into '{}': {e}",
                        install_sub.display(),
                        shared_sub.display()
                    )
                })?;
                log::info!(
                    "shared_data: merged {service_type}/{sub} into shared (shared wins on conflicts)"
                );
            }

            // Move the install dir aside so junction can take its name.
            let bak = install_dir.join(format!("{sub}.bak-{ts}"));
            std::fs::rename(&install_sub, &bak).map_err(|e| {
                format!(
                    "Failed to rename '{}' aside as '{}': {e}",
                    install_sub.display(),
                    bak.display()
                )
            })?;
        }

        create_junction(&install_sub, &shared_sub).map_err(|e| {
            format!(
                "Failed to junction '{}' -> '{}': {e}",
                install_sub.display(),
                shared_sub.display()
            )
        })?;

        wired.push(format!("{service_type}/{sub}"));
    }

    Ok(wired)
}

fn is_junction(path: &Path) -> bool {
    std::fs::read_link(path).is_ok()
}

fn directory_has_files(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    match std::fs::read_dir(path) {
        Ok(mut entries) => Ok(entries.next().is_some()),
        Err(e) => Err(format!("read_dir({}): {e}", path.display())),
    }
}

#[allow(dead_code)] // kept for explicit "copy everything" callers; merge_into_shared is the safe default
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ft.is_file() {
            std::fs::copy(&from, &to)?;
        }
        // Skip symlinks/junctions silently.
    }
    Ok(())
}

/// Recursively copy entries from `src` to `dst`, but ONLY for paths that
/// don't already exist in `dst`. Used during shared-data wiring to recover
/// user-authored files from an install dir we're about to junction over —
/// without ever overwriting whatever is already canonical in `.shared/`.
fn merge_into_shared(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            // Recurse so a partial overlap (e.g. shared has nginx.conf but
            // not sites-enabled/) still picks up missing subtrees.
            merge_into_shared(&from, &to)?;
        } else if ft.is_file() {
            if to.exists() {
                continue; // shared wins
            }
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn create_junction(link: &Path, target: &Path) -> Result<(), String> {
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
    std::os::unix::fs::symlink(target, link).map_err(|e| {
        format!(
            "Failed to symlink '{}' -> '{}': {e}",
            link.display(),
            target.display()
        )
    })
}
