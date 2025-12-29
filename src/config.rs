use serde::{Deserialize, Serialize};
use std::env;
use ve_energy_scrapers::models::strategy_information_scraper_config::StrategyInformationScraperConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScraperConfig {
    #[serde(flatten)]
    pub scraper_config: StrategyInformationScraperConfig,
    pub sub_data_folder: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_prefix: Option<String>,
    pub scrapers: Vec<ScraperConfig>,
    pub retention_days: Option<u64>,
}

impl AppConfig {
    /// Get S3 bucket from env var S3_BUCKET, falling back to config file
    pub fn get_s3_bucket(&self) -> Option<String> {
        env::var("S3_BUCKET").ok().or_else(|| self.s3_bucket.clone())
    }
    
    /// Get S3 region from env var S3_REGION, falling back to config file
    pub fn get_s3_region(&self) -> Option<String> {
        env::var("S3_REGION").ok().or_else(|| self.s3_region.clone())
    }
    
    /// Get S3 endpoint from env var S3_ENDPOINT, falling back to config file
    pub fn get_s3_endpoint(&self) -> Option<String> {
        env::var("S3_ENDPOINT").ok().or_else(|| self.s3_endpoint.clone())
    }
    
    /// Get S3 prefix from env var S3_PREFIX, falling back to config file, default "data/"
    pub fn get_s3_prefix(&self) -> String {
        env::var("S3_PREFIX")
            .ok()
            .or_else(|| self.s3_prefix.clone())
            .unwrap_or_else(|| "data/".to_string())
    }
}

pub fn load_config(path: &str) -> anyhow::Result<AppConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: AppConfig = serde_json::from_str(&content)?;
    Ok(config)
}
