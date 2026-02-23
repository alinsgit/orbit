use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

use super::hidden_command;

/// Manages site-level application processes (e.g., `npm run dev`, `python manage.py runserver`).
/// These are distinct from service processes (nginx, mariadb, etc.) — they run per-site
/// and are tied to a site's `dev_command` field.
pub struct SiteProcessManager {
    processes: Arc<Mutex<HashMap<String, Child>>>,
}

impl SiteProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start a site's app process using its dev_command.
    /// `domain` — site domain (used as key)
    /// `dev_command` — the command to run (e.g., "npm run dev")
    /// `working_dir` — the site's root path
    /// `dev_port` — optional port override (injected as PORT env var)
    pub fn start(
        &self,
        domain: &str,
        dev_command: &str,
        working_dir: &str,
        dev_port: Option<u16>,
    ) -> Result<u32, String> {
        // Check if already running
        {
            let mut processes = self.processes.lock().map_err(|e| e.to_string())?;
            if let Some(child) = processes.get_mut(domain) {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Process exited, remove it
                        processes.remove(domain);
                    }
                    Ok(None) => {
                        return Err(format!("Site app for {} is already running", domain));
                    }
                    Err(_) => {
                        processes.remove(domain);
                    }
                }
            }
        }

        // Parse the dev_command — split into program + args
        let parts: Vec<&str> = dev_command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("dev_command is empty".to_string());
        }

        let (program, args) = Self::resolve_command(&parts)?;

        let work_path = std::path::Path::new(working_dir);
        if !work_path.exists() {
            return Err(format!("Working directory does not exist: {}", working_dir));
        }

        let mut command = Command::new(&program);
        command.args(&args);
        command.current_dir(work_path);

        // Set PORT env var if dev_port is specified
        if let Some(port) = dev_port {
            command.env("PORT", port.to_string());
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        match command.spawn() {
            Ok(child) => {
                let pid = child.id();
                let mut processes = self.processes.lock().map_err(|e| e.to_string())?;
                processes.insert(domain.to_string(), child);
                log::info!("Started site app for {} (PID: {}, cmd: {})", domain, pid, dev_command);
                Ok(pid)
            }
            Err(e) => Err(format!("Failed to start site app: {}", e)),
        }
    }

    /// Stop a site's app process.
    pub fn stop(&self, domain: &str) -> Result<(), String> {
        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;

        if let Some(mut child) = processes.remove(domain) {
            let pid = child.id();

            #[cfg(target_os = "windows")]
            {
                let _ = hidden_command("taskkill")
                    .args(&["/F", "/PID", &pid.to_string(), "/T"])
                    .output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = Command::new("kill")
                    .args(&["-TERM", &pid.to_string()])
                    .output();
            }

            let _ = child.wait();
            log::info!("Stopped site app for {} (PID: {})", domain, pid);
            Ok(())
        } else {
            Err(format!("No running app process for site {}", domain))
        }
    }

    /// Get the status of a site's app process.
    /// Returns "running", "stopped", or "crashed".
    pub fn status(&self, domain: &str) -> String {
        let mut processes = match self.processes.lock() {
            Ok(p) => p,
            Err(_) => return "stopped".to_string(),
        };

        if let Some(child) = processes.get_mut(domain) {
            match child.try_wait() {
                Ok(Some(_)) => {
                    processes.remove(domain);
                    "crashed".to_string()
                }
                Ok(None) => "running".to_string(),
                Err(_) => {
                    processes.remove(domain);
                    "stopped".to_string()
                }
            }
        } else {
            "stopped".to_string()
        }
    }

    /// Stop all site app processes.
    pub fn stop_all(&self) -> Result<(), String> {
        let mut processes = self.processes.lock().map_err(|e| e.to_string())?;
        let domains: Vec<String> = processes.keys().cloned().collect();

        for domain in domains {
            if let Some(mut child) = processes.remove(&domain) {
                let pid = child.id();

                #[cfg(target_os = "windows")]
                {
                    let _ = hidden_command("taskkill")
                        .args(&["/F", "/PID", &pid.to_string(), "/T"])
                        .output();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = Command::new("kill")
                        .args(&["-TERM", &pid.to_string()])
                        .output();
                }

                let _ = child.wait();
                log::info!("Stopped site app for {} (PID: {})", domain, pid);
            }
        }

        Ok(())
    }

    /// Resolve a command like ["npm", "run", "dev"] into (program, args).
    /// On Windows, npm/npx/bun/python etc. need to go through cmd.exe or
    /// be resolved to their .cmd/.bat wrappers.
    fn resolve_command<'a>(parts: &'a [&'a str]) -> Result<(String, Vec<String>), String> {
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        let cmd = parts[0];
        let rest: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        // On Windows, commands like npm, npx, bun, python may be .cmd/.bat scripts
        // that need to be executed through cmd.exe
        #[cfg(target_os = "windows")]
        {
            let needs_shell = matches!(
                cmd.to_lowercase().as_str(),
                "npm" | "npx" | "yarn" | "pnpm" | "bun" | "bunx"
                    | "python" | "python3" | "pip" | "pip3"
                    | "composer" | "php"
                    | "deno" | "go"
                    | "node"
            );

            if needs_shell {
                let mut args = vec!["/C".to_string(), cmd.to_string()];
                args.extend(rest);
                return Ok(("cmd".to_string(), args));
            }
        }

        Ok((cmd.to_string(), rest))
    }
}
