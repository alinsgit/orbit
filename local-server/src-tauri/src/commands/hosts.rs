use crate::services::hosts::HostsManager;
use tauri::command;

#[command]
pub fn add_host(domain: String) -> Result<String, String> {
    match HostsManager::add_domain(&domain) {
        Ok(_) => Ok(format!("Domain {} added to hosts file", domain)),
        Err(e) => Err(e),
    }
}

#[command]
pub fn add_host_elevated(domain: String) -> Result<String, String> {
    match HostsManager::add_domain_elevated(&domain) {
        Ok(_) => Ok(format!("Domain {} added to hosts file", domain)),
        Err(e) => Err(e),
    }
}

#[command]
pub fn remove_host(domain: String) -> Result<String, String> {
    match HostsManager::remove_domain(&domain) {
        Ok(_) => Ok(format!("Domain {} removed from hosts file", domain)),
        Err(e) => Err(e),
    }
}

#[command]
#[allow(dead_code)]
pub fn check_admin() -> bool {
    HostsManager::check_admin()
}
