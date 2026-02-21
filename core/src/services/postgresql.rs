use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct PostgreSQLManager;

impl PostgreSQLManager {
    /// Initialize PostgreSQL database cluster
    pub fn initialize(
        postgres_root: &PathBuf,
        data_dir: &PathBuf,
        username: &str,
    ) -> Result<(), String> {
        // 1. Check if already initialized (PG_VERSION file exists)
        if data_dir.join("PG_VERSION").exists() {
            log::info!("PostgreSQL already initialized, skipping initdb");
            return Ok(());
        }

        // 2. Clean data directory from previous failed attempts
        if data_dir.exists() {
            fs::remove_dir_all(data_dir)
                .map_err(|e| format!("Failed to clean postgres data directory: {}", e))?;
        }
        fs::create_dir_all(data_dir)
            .map_err(|e| format!("Failed to create postgres data directory: {}", e))?;

        // 3. Run initdb
        let initdb_path = postgres_root.join("bin").join(if cfg!(windows) { "initdb.exe" } else { "initdb" });
        
        if !initdb_path.exists() {
            return Err(format!("initdb not found at {}", initdb_path.display()));
        }

        let mut cmd = Command::new(&initdb_path);
        cmd.arg("-D").arg(data_dir)
           .arg("-U").arg(username)
           .arg("--auth-local=trust")
           .arg("--auth-host=trust")
           .arg("--locale=C")
           .arg("--encoding=UTF8");

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = cmd.output()
            .map_err(|e| format!("Failed to execute initdb: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("initdb failed: {}\nStdout: {}", stderr, stdout));
        }

        log::info!("PostgreSQL initialized successfully");
        Ok(())
    }
}
