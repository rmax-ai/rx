use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct CliDefaults {
    pub max_iterations: Option<usize>,
    pub auto_commit: Option<bool>,
    pub resume: Option<String>,
    pub debug_log: Option<String>,
    pub list: Option<bool>,
    #[serde(alias = "model")]
    pub model_name: Option<String>,
}

impl CliDefaults {
    fn merge(self, overlay: CliDefaults) -> CliDefaults {
        CliDefaults {
            max_iterations: overlay.max_iterations.or(self.max_iterations),
            auto_commit: overlay.auto_commit.or(self.auto_commit),
            resume: overlay.resume.or(self.resume),
            debug_log: overlay.debug_log.or(self.debug_log),
            list: overlay.list.or(self.list),
            model_name: overlay.model_name.or(self.model_name),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    cli_defaults: Option<CliDefaults>,
    #[serde(flatten)]
    top_level: CliDefaults,
}

impl RawConfig {
    fn into_cli_defaults(self) -> CliDefaults {
        match self.cli_defaults {
            Some(cli_defaults) => self.top_level.merge(cli_defaults),
            None => self.top_level,
        }
    }
}

pub fn load_config<P: AsRef<Path>>(config_path: P) -> Result<CliDefaults> {
    if config_path.as_ref().exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {:?}", config_path.as_ref()))?;
        let raw: RawConfig = toml::from_str(&content).context("Invalid TOML in config file")?;
        Ok(raw.into_cli_defaults())
    } else {
        Ok(CliDefaults::default())
    }
}
