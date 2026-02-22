use crate::commands::scanner::get_installed_services;
use crate::commands::service::start_service;
use crate::services::process::ServiceManager;
use tauri::command;
use tauri::{AppHandle, State};

#[command]
pub fn auto_start_services(
    app: AppHandle,
    state: State<'_, ServiceManager>,
    installed_services: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut results = Vec::new();

    // Get all installed services with their bin paths from the scanner
    let all_services = get_installed_services(app.clone())?;

    for service_name in installed_services {
        // Find the matching installed service to get its bin_path
        let service = all_services.iter().find(|s| s.name == service_name);

        match service {
            Some(svc) => {
                match start_service(app.clone(), state.clone(), svc.name.clone(), svc.path.clone()) {
                    Ok(msg) => results.push(msg),
                    Err(e) => results.push(format!("Failed to start {}: {}", service_name, e)),
                }
            }
            None => {
                results.push(format!("Service {} not found in installed services", service_name));
            }
        }
    }

    Ok(results)
}
