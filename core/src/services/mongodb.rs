use std::fs;
use std::path::PathBuf;

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
}
