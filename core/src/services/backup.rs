use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::io::Write;

use super::hidden_command;

pub struct BackupManager;

impl BackupManager {
    /// Find mariadb-dump.exe or mysqldump.exe
    pub fn find_dump_exe(mariadb_root: &PathBuf) -> Result<PathBuf, String> {
        let paths = [
            mariadb_root.join("mariadb-dump.exe"),
            mariadb_root.join("mysqldump.exe"),
            mariadb_root.join("bin").join("mariadb-dump.exe"),
            mariadb_root.join("bin").join("mysqldump.exe"),
        ];

        for path in paths {
            if path.exists() {
                return Ok(path);
            }
        }

        Err("MariaDB dump executable not found (mariadb-dump.exe / mysqldump.exe)".to_string())
    }

    /// Find mysql.exe or mariadb.exe client
    pub fn find_client_exe(mariadb_root: &PathBuf) -> Result<PathBuf, String> {
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

        Err("MariaDB client executable not found (mariadb.exe / mysql.exe)".to_string())
    }

    /// Export a single database to a .sql file
    pub fn export_database(
        mariadb_root: &PathBuf,
        db_name: &str,
        output_path: &str,
    ) -> Result<String, String> {
        let dump_exe = Self::find_dump_exe(mariadb_root)?;

        log::info!("Exporting database '{}' to '{}'", db_name, output_path);

        let output = hidden_command(&dump_exe)
            .arg("--host=127.0.0.1")
            .arg("--port=3306")
            .arg("-u")
            .arg("root")
            .arg("-proot")
            .arg("--routines")
            .arg("--triggers")
            .arg("--single-transaction")
            .arg(db_name)
            .output()
            .map_err(|e| format!("Failed to run dump: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Database export failed: {}", stderr));
        }

        // Write stdout (the SQL dump) to the output file
        fs::write(output_path, &output.stdout)
            .map_err(|e| format!("Failed to write dump file: {}", e))?;

        let size = output.stdout.len();
        log::info!("Export complete: {} bytes written to '{}'", size, output_path);

        Ok(format!(
            "Database '{}' exported successfully ({} bytes)",
            db_name, size
        ))
    }

    /// Export all databases to a single .sql file
    pub fn export_all_databases(
        mariadb_root: &PathBuf,
        output_path: &str,
    ) -> Result<String, String> {
        let dump_exe = Self::find_dump_exe(mariadb_root)?;

        log::info!("Exporting all databases to '{}'", output_path);

        let output = hidden_command(&dump_exe)
            .arg("--host=127.0.0.1")
            .arg("--port=3306")
            .arg("-u")
            .arg("root")
            .arg("-proot")
            .arg("--all-databases")
            .arg("--routines")
            .arg("--triggers")
            .arg("--single-transaction")
            .output()
            .map_err(|e| format!("Failed to run dump: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Full export failed: {}", stderr));
        }

        fs::write(output_path, &output.stdout)
            .map_err(|e| format!("Failed to write dump file: {}", e))?;

        let size = output.stdout.len();
        log::info!(
            "Full export complete: {} bytes written to '{}'",
            size,
            output_path
        );

        Ok(format!(
            "All databases exported successfully ({} bytes)",
            size
        ))
    }

    /// Import a .sql file into a database
    pub fn import_sql(
        mariadb_root: &PathBuf,
        db_name: &str,
        sql_path: &str,
    ) -> Result<String, String> {
        let client_exe = Self::find_client_exe(mariadb_root)?;

        // Validate file exists
        if !std::path::Path::new(sql_path).exists() {
            return Err(format!("SQL file not found: {}", sql_path));
        }

        // Read the SQL file
        let sql_content = fs::read(sql_path)
            .map_err(|e| format!("Failed to read SQL file: {}", e))?;

        let file_size = sql_content.len();
        log::info!(
            "Importing '{}' ({} bytes) into database '{}'",
            sql_path,
            file_size,
            db_name
        );

        // Pipe the SQL content via stdin
        let mut child = hidden_command(&client_exe)
            .arg("--host=127.0.0.1")
            .arg("--port=3306")
            .arg("-u")
            .arg("root")
            .arg("-proot")
            .arg(db_name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start mysql client: {}", e))?;

        // Write SQL to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&sql_content)
                .map_err(|e| format!("Failed to write to mysql stdin: {}", e))?;
            // stdin is dropped here, closing it
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for mysql client: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("SQL import failed: {}", stderr));
        }

        log::info!("Import complete for database '{}'", db_name);

        Ok(format!(
            "SQL file imported successfully into '{}' ({} bytes)",
            db_name, file_size
        ))
    }

    /// Rebuild a database: drop → create → import
    pub fn rebuild_database(
        mariadb_root: &PathBuf,
        db_name: &str,
        sql_path: &str,
    ) -> Result<String, String> {
        let client_exe = Self::find_client_exe(mariadb_root)?;

        // Prevent rebuilding system databases
        let system_dbs = ["mysql", "information_schema", "performance_schema", "sys"];
        if system_dbs.contains(&db_name.to_lowercase().as_str()) {
            return Err(format!("Cannot rebuild system database '{}'", db_name));
        }

        // Validate SQL file exists
        if !std::path::Path::new(sql_path).exists() {
            return Err(format!("SQL file not found: {}", sql_path));
        }

        log::info!("Rebuilding database '{}' from '{}'", db_name, sql_path);

        // Step 1: Drop database
        let drop_sql = format!("DROP DATABASE IF EXISTS `{}`", db_name);
        let output = hidden_command(&client_exe)
            .arg("--host=127.0.0.1")
            .arg("--port=3306")
            .arg("-u")
            .arg("root")
            .arg("-proot")
            .arg("-e")
            .arg(&drop_sql)
            .output()
            .map_err(|e| format!("Failed to drop database: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to drop database '{}': {}", db_name, stderr));
        }

        log::info!("Dropped database '{}'", db_name);

        // Step 2: Create database
        let create_sql = format!(
            "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
            db_name
        );
        let output = hidden_command(&client_exe)
            .arg("--host=127.0.0.1")
            .arg("--port=3306")
            .arg("-u")
            .arg("root")
            .arg("-proot")
            .arg("-e")
            .arg(&create_sql)
            .output()
            .map_err(|e| format!("Failed to create database: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Failed to create database '{}': {}",
                db_name, stderr
            ));
        }

        log::info!("Created database '{}'", db_name);

        // Step 3: Import SQL file
        Self::import_sql(mariadb_root, db_name, sql_path)?;

        log::info!("Rebuild complete for database '{}'", db_name);

        Ok(format!("Database '{}' rebuilt successfully", db_name))
    }
}
