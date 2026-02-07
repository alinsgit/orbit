use crate::services::process::ServiceManager;
use tauri::command;
use tauri::State;

#[command]
pub fn auto_start_services(
    state: State<'_, ServiceManager>,
    installed_services: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut results = Vec::new();

    // Check if auto-start is enabled
    // For now, we'll start all services if they exist
    for service_name in installed_services {
        match state.start_auto(service_name.clone()) {
            Ok(pid) => {
                results.push(format!("Started {} with PID {}", service_name, pid));
            }
            Err(e) => {
                results.push(format!("Failed to start {}: {}", service_name, e));
            }
        }
    }

    Ok(results)
}
