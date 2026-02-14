use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct CliDefaults {  // Make this struct public
    pub max_iterations: Option<usize>,
    pub auto_commit: Option<bool>,
    pub resume: Option<String>,
    pub debug_log: Option<String>,
    pub list: Option<bool>,
    pub model_name: Option<String>, // New field for model_name
}

pub fn load_config<P: AsRef<Path>>(config_path: P) -> Result<CliDefaults> {
    if config_path.as_ref().exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {:?}", config_path.as_ref()))?;
        toml::from_str(&content).context("Invalid TOML in config file")
    } else {
        Ok(CliDefaults::default())
    }
}
