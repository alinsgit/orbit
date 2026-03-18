use crate::services::php_registry::{PhpRegistry, PhpService};
use tauri::{command, AppHandle};

/// Get all registered PHP services
#[command]
pub fn get_php_services(app: AppHandle) -> Result<Vec<PhpService>, String> {
    let mut registry = PhpRegistry::load(&app)?;

    // Verify running services (check PIDs)
    registry.verify_running_services();
    registry.save(&app)?;

    Ok(registry.services)
}

/// Get a specific PHP service by version
#[command]
pub fn get_php_service(app: AppHandle, version: String) -> Result<Option<PhpService>, String> {
    let registry = PhpRegistry::load(&app)?;
    Ok(registry.get_service(&version).cloned())
}

/// Get PHP port for a version
#[command]
pub fn get_php_port(app: AppHandle, version: String) -> Result<u16, String> {
    let registry = PhpRegistry::load(&app)?;
    Ok(registry.get_or_calculate_port(&version))
}

/// Register a PHP version
#[command]
pub fn register_php_version(app: AppHandle, version: String, path: String) -> Result<PhpService, String> {
    let mut registry = PhpRegistry::load(&app)?;
    let service = registry.register_php(&version, &path).clone();
    registry.save(&app)?;
    Ok(service)
}

/// Unregister a PHP version
#[command]
pub fn unregister_php_version(app: AppHandle, version: String) -> Result<bool, String> {
    let mut registry = PhpRegistry::load(&app)?;
    let result = registry.unregister_php(&version);
    registry.save(&app)?;
    Ok(result)
}

/// Mark PHP service as running
#[command]
pub fn mark_php_running(app: AppHandle, version: String, pid: u32) -> Result<bool, String> {
    let mut registry = PhpRegistry::load(&app)?;
    let result = registry.mark_running(&version, pid);
    registry.save(&app)?;
    Ok(result)
}

/// Mark PHP service as stopped
#[command]
pub fn mark_php_stopped(app: AppHandle, version: String) -> Result<bool, String> {
    let mut registry = PhpRegistry::load(&app)?;
    let result = registry.mark_stopped(&version);
    registry.save(&app)?;
    Ok(result)
}

/// Scan and register installed PHP versions
#[command]
pub fn scan_php_versions(app: AppHandle) -> Result<usize, String> {
    let mut registry = PhpRegistry::load(&app)?;
    let count = registry.scan_installed_versions(&app)?;
    registry.save(&app)?;
    Ok(count)
}

/// Get running PHP services
#[command]
pub fn get_running_php_services(app: AppHandle) -> Result<Vec<PhpService>, String> {
    let mut registry = PhpRegistry::load(&app)?;
    registry.verify_running_services();
    registry.save(&app)?;

    Ok(registry.get_running_services().into_iter().cloned().collect())
}

/// Calculate port for a PHP version (without saving)
#[command]
pub fn calculate_php_port(version: String) -> u16 {
    PhpRegistry::calculate_port(&version)
}
