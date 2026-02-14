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

    pub fn merge(self, overlay: CliDefaults) -> CliDefaults {
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

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub model: Option<String>,
    pub cli_defaults_overrides: Option<CliDefaults>,
}

#[derive(Debug, Clone)]
pub enum AgentConfigState {
    Valid(AgentConfig),
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub cli_defaults: CliDefaults,
    pub agent: Option<AgentConfigState>,
}

impl Default for LoadedConfig {
    fn default() -> Self {
        LoadedConfig {
            cli_defaults: CliDefaults::default(),
            agent: None,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    cli_defaults: Option<CliDefaults>,
    #[serde(flatten)]
    top_level: CliDefaults,
    #[serde(default)]
    agent: Option<RawAgentConfig>,
}

impl RawConfig {
    fn into_components(self) -> (CliDefaults, Option<RawAgentConfig>) {
        let cli_defaults = match self.cli_defaults {
            Some(cli_defaults) => self.top_level.merge(cli_defaults),
            None => self.top_level,
        };
        (cli_defaults, self.agent)
    }

    fn into_cli_defaults(self) -> CliDefaults {
        self.into_components().0
    }
}

#[derive(Debug, Deserialize, Clone)]
struct RawAgentConfig {
    name: Option<String>,
    model: Option<String>,
    #[serde(default)]
    cli_defaults_overrides: Option<CliDefaults>,
}

impl RawAgentConfig {
    fn into_state(self) -> AgentConfigState {
        let trimmed_name = self.name.and_then(|name| {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        match trimmed_name {
            Some(name) => AgentConfigState::Valid(AgentConfig {
                name,
                model: self.model,
                cli_defaults_overrides: self.cli_defaults_overrides,
            }),
            None => {
                AgentConfigState::Invalid("agent.name must be provided and non-empty".to_string())
            }
        }
    }
}

pub fn load_config<P: AsRef<Path>>(config_path: P) -> Result<LoadedConfig> {
    if config_path.as_ref().exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {:?}", config_path.as_ref()))?;
        let raw: RawConfig = toml::from_str(&content).context("Invalid TOML in config file")?;
        let (cli_defaults, agent) = raw.into_components();
        Ok(LoadedConfig {
            cli_defaults,
            agent: agent.map(|agent| agent.into_state()),
        })
    } else {
        Ok(LoadedConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentConfigState, RawConfig};
    use toml;

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
        assert_eq!(
            defaults.resolved_small_model().as_deref(),
            Some("gpt-5-mini")
        );
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
        assert_eq!(
            defaults.resolved_small_model().as_deref(),
            Some("gpt-5-mini")
        );
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
        assert_eq!(
            defaults.resolved_small_model().as_deref(),
            Some("gpt-5-mini")
        );
        assert!(!defaults.uses_legacy_auto_commit_model());
    }

    #[test]
    fn parses_valid_agent_section_with_defaults_overrides() {
        let raw: RawConfig = toml::from_str(
            r#"
            [agent]
            name = "writer"
            model = "gpt-5.3-codex"

            [agent.cli_defaults_overrides]
            max_iterations = 80
            tool_verbose = true
            "#,
        )
        .expect("parses toml");

        let (_, agent_section) = raw.into_components();
        let agent_state = agent_section
            .expect("agent section present")
            .into_state();

        match agent_state {
            AgentConfigState::Valid(agent) => {
                assert_eq!(agent.name, "writer");
                assert_eq!(agent.model.as_deref(), Some("gpt-5.3-codex"));
                let overrides = agent
                    .cli_defaults_overrides
                    .expect("overrides present");
                assert_eq!(overrides.max_iterations, Some(80));
                assert_eq!(overrides.tool_verbose, Some(true));
            }
            AgentConfigState::Invalid(reason) => panic!("unexpected invalid agent: {}", reason),
        }
    }

    #[test]
    fn invalid_agent_name_returns_invalid_state() {
        let raw: RawConfig = toml::from_str(
            r#"
            [agent]
            name = "   "
            "#,
        )
        .expect("parses toml");

        let (_, agent_section) = raw.into_components();
        let agent_state = agent_section
            .expect("agent section present")
            .into_state();

        match agent_state {
            AgentConfigState::Valid(agent) => panic!("expected invalid agent but got valid={:?}", agent),
            AgentConfigState::Invalid(reason) => {
                assert_eq!(reason, "agent.name must be provided and non-empty");
            }
        }
    }
}
