use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
struct CliDefaults {
    max_iterations: Option<usize>,
    auto_commit: Option<bool>,
    resume: Option<String>,
    debug_log: Option<String>,
    list: Option<bool>,
}

pub fn load_config<P: AsRef<Path>>(config_path: P) -> Result<CliDefaults> {
    if config_path.as_ref().exists() {
        let content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file at {:?}", config_path.as_ref()))?;
        toml::from_str(&content).context("Invalid TOML in config file")
    } else {
        Ok(CliDefaults::default())
    }
}
