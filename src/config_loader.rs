use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct CliDefaults {
    pub max_iterations: Option<usize>,
    pub auto_commit: Option<bool>,
    pub small_model: Option<String>,
    #[serde(alias = "autocommit_model")]
    pub auto_commit_model: Option<String>,
    pub resume: Option<String>,
    pub debug_log: Option<String>,
    pub list: Option<bool>,
    #[serde(alias = "model")]
    pub model_name: Option<String>,
    pub tool_verbose: Option<bool>,
}

impl CliDefaults {
    pub fn resolved_small_model(&self) -> Option<String> {
        self.small_model
            .clone()
            .or_else(|| self.auto_commit_model.clone())
    }

    pub fn uses_legacy_auto_commit_model(&self) -> bool {
        self.small_model.is_none() && self.auto_commit_model.is_some()
    }

    fn merge(self, overlay: CliDefaults) -> CliDefaults {
        CliDefaults {
            max_iterations: overlay.max_iterations.or(self.max_iterations),
            auto_commit: overlay.auto_commit.or(self.auto_commit),
            small_model: overlay.small_model.or(self.small_model),
            auto_commit_model: overlay.auto_commit_model.or(self.auto_commit_model),
            resume: overlay.resume.or(self.resume),
            debug_log: overlay.debug_log.or(self.debug_log),
            list: overlay.list.or(self.list),
            model_name: overlay.model_name.or(self.model_name),
            tool_verbose: overlay.tool_verbose.or(self.tool_verbose),
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

#[cfg(test)]
mod tests {
    use super::RawConfig;

    #[test]
    fn resolves_small_model_from_small_model_field() {
        let raw: RawConfig = toml::from_str(
            r#"
            [cli_defaults]
            small_model = "gpt-5-mini"
            "#,
        )
        .expect("parses toml");

        let defaults = raw.into_cli_defaults();
        assert_eq!(defaults.resolved_small_model().as_deref(), Some("gpt-5-mini"));
        assert!(!defaults.uses_legacy_auto_commit_model());
    }

    #[test]
    fn resolves_small_model_from_legacy_auto_commit_model_field() {
        let raw: RawConfig = toml::from_str(
            r#"
            [cli_defaults]
            auto_commit_model = "gpt-5-mini"
            "#,
        )
        .expect("parses toml");

        let defaults = raw.into_cli_defaults();
        assert_eq!(defaults.resolved_small_model().as_deref(), Some("gpt-5-mini"));
        assert!(defaults.uses_legacy_auto_commit_model());
    }

    #[test]
    fn prefers_small_model_over_legacy_field_when_both_are_set() {
        let raw: RawConfig = toml::from_str(
            r#"
            [cli_defaults]
            small_model = "gpt-5-mini"
            auto_commit_model = "gpt-4o-mini"
            "#,
        )
        .expect("parses toml");

        let defaults = raw.into_cli_defaults();
        assert_eq!(defaults.resolved_small_model().as_deref(), Some("gpt-5-mini"));
        assert!(!defaults.uses_legacy_auto_commit_model());
    }
}
