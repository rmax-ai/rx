use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

pub const AVAILABLE_TOOLS: [&str; 10] = [
    "exec",
    "read_file",
    "write_file",
    "create_file",
    "append_file",
    "replace_in_file",
    "apply_patch",
    "apply_unified_patch",
    "list_dir",
    "done",
];

#[derive(Debug, Deserialize, Default)]
pub struct RxConfig {
    pub tools: Option<ToolsConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ToolsConfig {
    pub enabled: Option<Vec<String>>,
    pub disabled: Option<Vec<String>>,
}

#[derive(Debug, Default)]
pub struct ToolSelection {
    pub enabled_tools: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn load_config(path: &Path) -> Option<RxConfig> {
    let raw = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "Warning: failed to read config file at {}: {}",
                    path.display(),
                    error
                );
            }
            return None;
        }
    };

    match toml::from_str::<RxConfig>(&raw) {
        Ok(config) => Some(config),
        Err(error) => {
            eprintln!(
                "Warning: failed to parse config file at {}: {}",
                path.display(),
                error
            );
            None
        }
    }
}

pub fn resolve_enabled_tools(config: Option<&ToolsConfig>) -> ToolSelection {
    let mut warnings = Vec::new();
    let available_set: HashSet<&str> = AVAILABLE_TOOLS.iter().copied().collect();
    let mut selected: Vec<String> = match config.and_then(|c| c.enabled.as_ref()) {
        Some(enabled) => {
            let enabled_set = to_trimmed_set(enabled);
            if enabled_set.is_empty() {
                warnings.push(
                    "Config [tools].enabled is empty; no tools selected before safety checks."
                        .to_string(),
                );
            }

            for name in &enabled_set {
                if !available_set.contains(name.as_str()) {
                    warnings.push(format!(
                        "Config [tools].enabled contains unknown tool '{}'; ignoring.",
                        name
                    ));
                }
            }

            AVAILABLE_TOOLS
                .iter()
                .filter(|name| enabled_set.contains(**name))
                .map(|name| (*name).to_string())
                .collect()
        }
        None => AVAILABLE_TOOLS
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
    };

    if let Some(disabled) = config.and_then(|c| c.disabled.as_ref()) {
        let disabled_set = to_trimmed_set(disabled);
        for name in &disabled_set {
            if !available_set.contains(name.as_str()) {
                warnings.push(format!(
                    "Config [tools].disabled contains unknown tool '{}'; ignoring.",
                    name
                ));
            }
        }

        selected.retain(|name| !disabled_set.contains(name));
    }

    if !selected.iter().any(|name| name == "done") {
        warnings
            .push("Tool 'done' cannot be disabled; forcing it to remain registered.".to_string());
        selected.push("done".to_string());
    }

    ToolSelection {
        enabled_tools: selected,
        warnings,
    }
}

fn to_trimmed_set(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{resolve_enabled_tools, ToolsConfig, AVAILABLE_TOOLS};
    use crate::config::load_config;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("rx-config-test-{}-{}.toml", name, nanos))
    }

    #[test]
    fn defaults_to_all_tools() {
        let selected = resolve_enabled_tools(None);
        assert_eq!(
            selected.enabled_tools,
            AVAILABLE_TOOLS
                .iter()
                .map(|name| (*name).to_string())
                .collect::<Vec<String>>()
        );
        assert!(selected.warnings.is_empty());
    }

    #[test]
    fn enabled_list_limits_tools() {
        let cfg = ToolsConfig {
            enabled: Some(vec!["read_file".to_string(), "done".to_string()]),
            disabled: None,
        };
        let selected = resolve_enabled_tools(Some(&cfg));
        assert_eq!(
            selected.enabled_tools,
            vec!["read_file".to_string(), "done".to_string()]
        );
        assert!(selected.warnings.is_empty());
    }

    #[test]
    fn done_is_forced_even_when_disabled() {
        let cfg = ToolsConfig {
            enabled: Some(vec!["exec".to_string()]),
            disabled: Some(vec!["done".to_string()]),
        };
        let selected = resolve_enabled_tools(Some(&cfg));
        assert_eq!(
            selected.enabled_tools,
            vec!["exec".to_string(), "done".to_string()]
        );
        assert_eq!(selected.warnings.len(), 1);
    }

    #[test]
    fn unknown_names_warn_and_are_ignored() {
        let cfg = ToolsConfig {
            enabled: Some(vec!["read_file".to_string(), "not_real".to_string()]),
            disabled: Some(vec!["also_fake".to_string()]),
        };
        let selected = resolve_enabled_tools(Some(&cfg));
        assert_eq!(
            selected.enabled_tools,
            vec!["read_file".to_string(), "done".to_string()]
        );
        assert_eq!(selected.warnings.len(), 3);
    }

    #[test]
    fn disabled_is_applied_after_enabled() {
        let cfg = ToolsConfig {
            enabled: Some(vec!["exec".to_string(), "read_file".to_string()]),
            disabled: Some(vec!["exec".to_string()]),
        };
        let selected = resolve_enabled_tools(Some(&cfg));
        assert_eq!(
            selected.enabled_tools,
            vec!["read_file".to_string(), "done".to_string()]
        );
    }

    #[test]
    fn load_config_parses_tools_section() {
        let path = temp_config_path("valid");
        fs::write(
            &path,
            r#"
[tools]
enabled = ["read_file", "done"]
disabled = ["exec"]
"#,
        )
        .expect("should write test config");

        let loaded = load_config(&path).expect("config should parse");
        let tools = loaded.tools.expect("tools section should exist");
        assert_eq!(
            tools.enabled.expect("enabled should exist"),
            vec!["read_file".to_string(), "done".to_string()]
        );
        assert_eq!(
            tools.disabled.expect("disabled should exist"),
            vec!["exec".to_string()]
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_config_returns_none_for_invalid_toml() {
        let path = temp_config_path("invalid");
        fs::write(&path, "[tools\nenabled = [\"read_file\"]").expect("should write test config");

        let loaded = load_config(&path);
        assert!(loaded.is_none());

        let _ = fs::remove_file(path);
    }
}
