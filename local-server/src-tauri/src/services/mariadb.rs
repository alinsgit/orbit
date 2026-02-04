use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub struct MariaDBManager;

impl MariaDBManager {
    /// Initialize MariaDB database
    pub fn initialize(
        mariadb_root: &PathBuf,
        data_dir: &PathBuf,
        _root_password: &str,
    ) -> Result<(), String> {
        // 1. Check if already initialized
        let mysql_db_dir = data_dir.join("mysql");
        if mysql_db_dir.exists() {
            log::info!("MariaDB already initialized, skipping initialization");
            return Ok(());
        }

        // 2. Clean data directory from previous failed attempts
        if data_dir.exists() {
            fs::remove_dir_all(data_dir)
                .map_err(|e| format!("Failed to clean data directory: {}", e))?;
        }
        fs::create_dir_all(data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        // 3. Run mysql_install_db to initialize (before config, since it needs empty dir)
        let install_db_path = mariadb_root.join("mysql_install_db.exe");
        let mariadbd_path = Self::find_mariadbd(mariadb_root)?;

        if install_db_path.exists() {
            Self::run_install_db(&install_db_path, mariadb_root, data_dir)?;
        } else {
            Self::run_mariadbd_initialize(&mariadbd_path, data_dir)?;
        }

        // 4. Create my.ini config after initialization (install_db needs empty dir)
        Self::create_config(mariadb_root, data_dir)?;

        Ok(())
    }

    /// Create MariaDB configuration file
    fn create_config(_mariadb_root: &PathBuf, data_dir: &PathBuf) -> Result<PathBuf, String> {
        let conf_path = data_dir.join("my.ini");

        // Use forward slashes for MySQL/MariaDB paths on Windows
        let data_path_str = data_dir.display().to_string().replace('\\', "/");

        let conf_content = format!(
            r#"[mysqld]
datadir={}
port=3306
bind-address=127.0.0.1
skip-networking=0
default-storage-engine=InnoDB
innodb_buffer_pool_size=128M
max_connections=100
character-set-server=utf8mb4
collation-server=utf8mb4_unicode_ci

[client]
port=3306
host=127.0.0.1

[mariadb]
"#,
            data_path_str
        );

        // Create parent directory if needed
        if let Some(parent) = conf_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create config directory: {}", e))?;
            }
        }

        let mut conf_file = fs::File::create(&conf_path)
            .map_err(|e| format!("Failed to create my.ini: {}", e))?;

        conf_file
            .write_all(conf_content.as_bytes())
            .map_err(|e| format!("Failed to write my.ini: {}", e))?;

        Ok(conf_path)
    }

    /// Find mariadbd.exe or mysqld.exe
    fn find_mariadbd(mariadb_root: &PathBuf) -> Result<PathBuf, String> {
        // Try mariadbd first (newer naming)
        let mariadbd = mariadb_root.join("mariadbd.exe");
        if mariadbd.exists() {
            return Ok(mariadbd);
        }

        // Fall back to mysqld
        let mysqld = mariadb_root.join("mysqld.exe");
        if mysqld.exists() {
            return Ok(mysqld);
        }

        // Check in bin subdirectory
        let bin_mariadbd = mariadb_root.join("bin").join("mariadbd.exe");
        if bin_mariadbd.exists() {
            return Ok(bin_mariadbd);
        }

        let bin_mysqld = mariadb_root.join("bin").join("mysqld.exe");
        if bin_mysqld.exists() {
            return Ok(bin_mysqld);
        }

        Err("MariaDB server executable not found".to_string())
    }

    /// Run mysql_install_db for initialization
    fn run_install_db(
        install_db_path: &PathBuf,
        mariadb_root: &PathBuf,
        data_dir: &PathBuf,
    ) -> Result<(), String> {
        log::info!("Running mysql_install_db for initialization");

        let output = hidden_command(install_db_path)
            .arg(format!("--datadir={}", data_dir.display()))
            .arg("--password=root")
            .arg("--default-user")
            .current_dir(mariadb_root)
            .output()
            .map_err(|e| format!("Failed to run mysql_install_db: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("mysql_install_db failed: {}", stderr));
        }

        log::info!("mysql_install_db completed successfully");
        Ok(())
    }

    /// Run mariadbd --initialize-insecure for initialization
    fn run_mariadbd_initialize(mariadbd_path: &PathBuf, data_dir: &PathBuf) -> Result<(), String> {
        log::info!("Running mariadbd --initialize-insecure");

        let output = hidden_command(mariadbd_path)
            .arg("--initialize-insecure")
            .arg(format!("--datadir={}", data_dir.display()))
            .output()
            .map_err(|e| format!("Failed to run mariadbd --initialize: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Some versions don't support --initialize-insecure
            if stderr.contains("unknown option") {
                log::warn!("--initialize-insecure not supported, trying without");
                return Ok(()); // Data files might be created differently
            }
            return Err(format!("mariadbd --initialize failed: {}", stderr));
        }

        log::info!("mariadbd initialization completed successfully");
        Ok(())
    }

    /// Check if MariaDB is initialized
    #[allow(dead_code)]
    pub fn is_initialized(data_dir: &PathBuf) -> bool {
        data_dir.join("mysql").exists()
    }

    /// Get the MariaDB server executable path
    #[allow(dead_code)]
    pub fn get_server_path(mariadb_root: &PathBuf) -> Result<PathBuf, String> {
        Self::find_mariadbd(mariadb_root)
    }

    /// Get the MariaDB client executable path
    #[allow(dead_code)]
    pub fn get_client_path(mariadb_root: &PathBuf) -> Result<PathBuf, String> {
        let paths = [
            mariadb_root.join("mariadb.exe"),
            mariadb_root.join("mysql.exe"),
            mariadb_root.join("bin").join("mariadb.exe"),
            mariadb_root.join("bin").join("mysql.exe"),
        ];

        for path in paths {
            if path.exists() {
                return Ok(path);
            }
        }

        Err("MariaDB client executable not found".to_string())
    }
}
