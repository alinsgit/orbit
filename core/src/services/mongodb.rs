use std::fs;
use std::path::PathBuf;

use super::hidden_command;

pub struct MongoDBManager;

impl MongoDBManager {
    /// Initialize MongoDB database (just creates the dbpath)
    pub fn initialize(data_dir: &PathBuf) -> Result<(), String> {
        if !data_dir.exists() {
            fs::create_dir_all(data_dir)
                .map_err(|e| format!("Failed to create mongodb data directory: {}", e))?;
            log::info!("MongoDB data directory created.");
        }
        Ok(())
    }

    /// Find the mongosh (or legacy mongo) client binary
    pub fn find_mongosh_client(bin_dir: &std::path::Path) -> Result<PathBuf, String> {
        let paths = [
            bin_dir.join("mongodb").join("bin").join("mongosh.exe"),
            bin_dir.join("mongodb").join("mongosh.exe"),
            bin_dir.join("mongodb").join("bin").join("mongo.exe"),
        ];

        for p in &paths {
            if p.exists() {
                return Ok(p.clone());
            }
        }

        // Try PATH
        #[cfg(target_os = "windows")]
        {
            for name in &["mongosh", "mongo"] {
                if let Ok(output) = hidden_command("where").arg(name).output() {
                    if output.status.success() {
                        let path = String::from_utf8_lossy(&output.stdout);
                        if let Some(line) = path.lines().next() {
                            let p = line.trim();
                            if !p.is_empty() {
                                return Ok(PathBuf::from(p));
                            }
                        }
                    }
                }
            }
        }

        Err("mongosh/mongo client not found. Is MongoDB installed?".to_string())
    }

    /// Run a JavaScript command via mongosh against a database
    pub fn run_command(bin_dir: &std::path::Path, database: &str, js_command: &str) -> Result<String, String> {
        let mongosh = Self::find_mongosh_client(bin_dir)?;

        let output = hidden_command(&mongosh)
            .arg("--host").arg("127.0.0.1")
            .arg("--port").arg("27017")
            .arg("--quiet")
            .arg(database)
            .arg("--eval").arg(js_command)
            .output()
            .map_err(|e| format!("Failed to run mongosh: {}. Is MongoDB running?", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("MongoDB error: {}", stderr.trim()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// List all databases
    pub fn list_databases(bin_dir: &std::path::Path) -> Result<Vec<String>, String> {
        let output = Self::run_command(bin_dir, "admin", "JSON.stringify(db.adminCommand('listDatabases').databases.map(d => d.name))")?;

        // Parse JSON array of strings
        let names: Vec<String> = serde_json::from_str(&output)
            .map_err(|e| format!("Failed to parse database list: {} — raw: {}", e, output))?;

        Ok(names)
    }

    /// List collections in a database
    pub fn list_collections(bin_dir: &std::path::Path, database: &str) -> Result<Vec<String>, String> {
        if !database.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Invalid database name".to_string());
        }

        let output = Self::run_command(bin_dir, database, "JSON.stringify(db.getCollectionNames())")?;

        let names: Vec<String> = serde_json::from_str(&output)
            .map_err(|e| format!("Failed to parse collection list: {} — raw: {}", e, output))?;

        Ok(names)
    }

    /// Get database stats (size, collections count, etc.)
    pub fn get_db_stats(bin_dir: &std::path::Path, database: &str) -> Result<String, String> {
        if !database.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Invalid database name".to_string());
        }

        Self::run_command(bin_dir, database, "JSON.stringify(db.stats())")
    }

    /// Drop a database
    pub fn drop_database(bin_dir: &std::path::Path, database: &str) -> Result<String, String> {
        if !database.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Invalid database name".to_string());
        }

        let system_dbs = ["admin", "local", "config"];
        if system_dbs.contains(&database) {
            return Err(format!("Cannot drop system database '{}'", database));
        }

        Self::run_command(bin_dir, database, "JSON.stringify(db.dropDatabase())")
    }
}
