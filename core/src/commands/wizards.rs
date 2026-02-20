use tauri::AppHandle;
use std::process::Command;
use std::path::PathBuf;

#[tauri::command]
pub async fn scaffold_project(
    _app_handle: AppHandle,
    project_type: String,     // e.g., "nextjs", "laravel", "astro"
    project_name: String,     // e.g., "my-new-app"
    workspace_path: String,   // chosen by the user in Settings
) -> Result<String, String> {
    
    // We expect the frontend to pass absolute safe workspace path
    let work_dir = PathBuf::from(&workspace_path);
    if !work_dir.exists() {
        return Err(format!("Workspace directory '{}' does not exist.", workspace_path));
    }
    
    let npx_path = "npx";
    let composer_path = "composer";
    let php_path = "php";
    
    let mut command: Command;
    let mut args: Vec<String> = Vec::new();
    
    match project_type.as_str() {
        "nextjs" => {
            args.push(format!("{} create-next-app@latest {} --typescript --tailwind --eslint --app --src-dir --import-alias @/*", npx_path, project_name));
        },
        "nuxt" => {
            args.push(format!("{} nuxi@latest init {}", npx_path, project_name));
        },
        "vue" => {
            args.push(format!("{} create-vue@latest {} --yes", npx_path, project_name));
        },
        "astro" => {
            args.push(format!("{} create-astro@latest {} --yes", npx_path, project_name));
        },
        "laravel" => {
            args.push(format!("{} {} create-project laravel/laravel {}", php_path, composer_path, project_name));
        },
        "wordpress" => {
            let php_script = format!(
                "copy('https://wordpress.org/latest.zip', 'wp.zip'); \
                $zip = new ZipArchive; \
                if ($zip->open('wp.zip') === TRUE) {{ \
                    $zip->extractTo('.'); \
                    $zip->close(); \
                    rename('wordpress', '{}'); \
                    unlink('wp.zip'); \
                }}", project_name
            );
            args.push(format!("{} -r \"{}\"", php_path, php_script));
        },
        _ => return Err(format!("Unsupported project type: {}", project_type)),
    }
    
    if cfg!(windows) {
        command = Command::new("cmd");
        command.arg("/C").args(&args);
    } else {
        command = Command::new("sh");
        command.arg("-c").args(&args);
    }
    
    // Execute command within the Target Workspace Path
    command.current_dir(&work_dir);
    
    // Inherit the Orbit Path (Re-use logic from terminal backend if needed)
    // To ensure npx/composer uses Orbit's Node/PHP respectively
    let custom_path = crate::services::terminal::build_orbit_path(&_app_handle);
    command.env("PATH", &custom_path);
    
    // Run the scaffolding command synchronously but captured.
    // In the future for long tasks, emit progress events using PTY or IPC.
    let output = command.output()
        .map_err(|e| format!("Failed to start project creation: {}", e))?;
        
    if output.status.success() {
        Ok(format!("Successfully created project: {}", project_name))
    } else {
        let err_text = String::from_utf8_lossy(&output.stderr);
        Err(format!("Scaffolding failed: {}", err_text))
    }
}
