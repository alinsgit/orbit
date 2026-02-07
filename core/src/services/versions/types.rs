use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum VersionSource {
    Api,
    Cache,
    Fallback,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceVersion {
    pub version: String,
    pub download_url: String,
    pub filename: String,
    pub release_date: Option<String>,
    #[serde(default)]
    pub source: VersionSource,
}

impl Default for VersionSource {
    fn default() -> Self {
        VersionSource::Api
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedVersions {
    pub versions: Vec<ServiceVersion>,
    pub fetched_at: i64,
    pub service: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct VersionCache {
    pub services: HashMap<String, CachedVersions>,
}
