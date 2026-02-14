use crate::config_loader::{load_config, AgentConfigState, LoadedConfig};
use crate::debug_logger::DebugLogger;
use crate::event::Event;
use crate::kernel::Kernel;
use crate::model::{
    CommitMessageGenerator, GoalSlugGenerator, MockCommitMessageModel, MockGoalSlugModel,
    MockModel, Model, OpenAICommitMessageModel, OpenAIGoalSlugModel, OpenAIModel,
};
use crate::sqlite_state::SqliteStateStore;
use crate::state::StateStore;
use crate::tool::ToolRegistry;
use crate::tools::{
    bash::BashTool,
    done::DoneTool,
    exec::ExecTool,
    exec_capture::ExecCaptureTool,
    exec_status::ExecStatusTool,
    exec_with_input::ExecWithInputTool,
    fs::{ListDirTool, ReadFileTool, WriteFileTool},
    stat_file::StatFileTool,
    which_command::WhichCommandTool,
};
use anyhow::{anyhow, Context, Result};
use dirs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::fs;

pub mod config_loader;
pub mod debug_logger;
pub mod event;
pub mod kernel;
pub mod model;
pub mod sqlite_state;
pub mod state;
pub mod tool;
pub mod tools;

enum ConfigPathSource {
    Git(PathBuf),
    Home(PathBuf),
}

impl ConfigPathSource {
    fn path(&self) -> &Path {
        match self {
            ConfigPathSource::Git(path) | ConfigPathSource::Home(path) => path,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ConfigPathSource::Git(_) => "git workspace",
            ConfigPathSource::Home(_) => "home directory",
        }
    }
}

fn resolve_config_path() -> ConfigPathSource {
    if let Some(git_config_path) = git_config_path() {
        if git_config_path.exists() {
            return ConfigPathSource::Git(git_config_path);
        }
    }

    ConfigPathSource::Home(home_config_path())
}

fn git_config_path() -> Option<PathBuf> {
    detect_git_root().map(|root| root.join(".rx/config.toml"))
}

fn detect_git_root() -> Option<PathBuf> {
    if let Some(git_root) = std::env::var_os("GIT_ROOT") {
        return Some(PathBuf::from(git_root));
    }

    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(PathBuf::from(trimmed))
}

fn home_config_path() -> PathBuf {
    dirs::home_dir()
        .map(|dir| dir.join(".rx/config.toml"))
        .unwrap_or_else(|| PathBuf::from(".rx/config.toml"))
}

fn expand_debug_log_path(template: &str, goal_id: &str) -> PathBuf {
    PathBuf::from(template.replace("{goal_id}", goal_id))
}

#[derive(Default)]
struct ParsedCliArgs {
    max_iterations: Option<usize>,
    auto_commit: bool,
    resume: Option<String>,
    debug_log: Option<String>,
    list: bool,
    tool_verbose: bool,
    model: Option<String>,
    small_model: Option<String>,
    agent: Option<String>,
    goal_parts: Vec<String>,
}

fn parse_cli_args() -> ParsedCliArgs {
    let mut parsed = ParsedCliArgs::default();
    let mut args_iter = std::env::args().skip(1);

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--max-iterations" => {
                let value = expect_flag_value(&mut args_iter, "--max-iterations");
                if let Ok(parsed_value) = value.parse::<usize>() {
                    parsed.max_iterations = Some(parsed_value);
                } else {
                    eprintln!(
                        "Warning: invalid value '{}' for --max-iterations. Ignoring.",
                        value
                    );
                }
            }
            "--auto-commit" => {
                parsed.auto_commit = true;
            }
            "--resume" => {
                parsed.resume = Some(expect_flag_value(&mut args_iter, "--resume"));
            }
            "--debug-log" => {
                parsed.debug_log = Some(expect_flag_value(&mut args_iter, "--debug-log"));
            }
            "--list" => {
                parsed.list = true;
            }
            "--tool-verbose" => {
                parsed.tool_verbose = true;
            }
            "--model" => {
                parsed.model = Some(expect_flag_value(&mut args_iter, "--model"));
            }
            "--small-model" => {
                parsed.small_model = Some(expect_flag_value(&mut args_iter, "--small-model"));
            }
            "--agent" => {
                let value = expect_flag_value(&mut args_iter, "--agent");
                if value.trim().is_empty() {
                    eprintln!("--agent flag requires a non-empty value.");
                    std::process::exit(1);
                }
                parsed.agent = Some(value);
            }
            other => {
                parsed.goal_parts.push(other.to_string());
            }
        }
    }

    parsed
}

fn expect_flag_value<I: Iterator<Item = String>>(args_iter: &mut I, flag: &str) -> String {
    args_iter.next().unwrap_or_else(|| {
        eprintln!("{} flag requires a value.", flag);
        std::process::exit(1);
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let ParsedCliArgs {
        max_iterations: cli_max_iterations,
        auto_commit: cli_auto_commit,
        resume: cli_resume,
        debug_log: cli_debug_log,
        list: cli_list,
        tool_verbose: cli_tool_verbose,
        model: cli_model,
        small_model: cli_small_model,
        agent: cli_agent,
        goal_parts,
    } = parse_cli_args();

    // Load Config
    let config_path_source = resolve_config_path();
    let config_path = config_path_source.path().to_path_buf();
    let config_description = format!(
        "{} ({})",
        config_path_source.description(),
        config_path.display()
    );
    let (loaded_config, config_source) = match load_config(&config_path) {
        Ok(config) => (config, format!("loaded from {}", config_description)),
        Err(err) => {
            eprintln!(
                "Warning: failed to load config from {}: {}. Using defaults.",
                config_path.display(),
                err
            );
            (
                LoadedConfig::default(),
                format!("defaults (config load failed at {})", config_description),
            )
        }
    };

    let mut effective_defaults = loaded_config.cli_defaults.clone();
    let agent_state = loaded_config.agent.clone();
    let mut matched_agent_model: Option<String> = None;

    if let Some(requested) = cli_agent.as_deref() {
        eprintln!("agent.requested: {}", requested);
        match agent_state.as_ref() {
            Some(AgentConfigState::Valid(agent)) if agent.name == requested => {
                let mut overrides_applied = false;
                if let Some(overrides) = &agent.cli_defaults_overrides {
                    effective_defaults = effective_defaults.merge(overrides.clone());
                    overrides_applied = true;
                }
                eprintln!(
                    "agent.matched: {} overrides_applied={}",
                    requested, overrides_applied
                );
                matched_agent_model = agent.model.clone();
            }
            Some(AgentConfigState::Valid(_)) => {
                return Err(anyhow!("Agent profile \"{}\" not found", requested));
            }
            Some(AgentConfigState::Invalid(reason)) => {
                return Err(anyhow!(
                    "Invalid agent config requested: {} ({})",
                    requested,
                    reason
                ));
            }
            None => {
                return Err(anyhow!("Agent profile \"{}\" not found", requested));
            }
        }
    } else if let Some(AgentConfigState::Invalid(reason)) = agent_state.as_ref() {
        eprintln!(
            "agent.config.invalid: reason={} (ignoring agent overlay)",
            reason
        );
    }

    let goal_id_to_resume = cli_resume;
    let max_iterations =
        cli_max_iterations.unwrap_or_else(|| effective_defaults.max_iterations.unwrap_or(50));
    let auto_commit = cli_auto_commit || effective_defaults.auto_commit.unwrap_or(false);
    let mut small_model = cli_small_model
        .clone()
        .or_else(|| effective_defaults.resolved_small_model());
    let small_model_from_legacy_config = effective_defaults.uses_legacy_auto_commit_model();
    let debug_log_template = cli_debug_log.or(effective_defaults.debug_log.clone());
    let list_goals = cli_list || effective_defaults.list.unwrap_or(false);
    let tool_verbose = cli_tool_verbose || effective_defaults.tool_verbose.unwrap_or(false);

    let mut model_name = cli_model
        .clone()
        .or_else(|| matched_agent_model.clone())
        .or_else(|| effective_defaults.model_name.clone());
    let model_set_by_cli = cli_model.is_some();
    if !model_set_by_cli {
        if let Ok(env_model_name) = std::env::var("OPENAI_MODEL") {
            if model_name.is_none() {
                model_name = Some(env_model_name);
            }
        }
    }
    let model_name = model_name.unwrap_or_else(|| "gpt-4o".to_string());

    if auto_commit && small_model.is_none() {
        small_model = Some("gpt-5-mini".to_string());
    }

    let api_key = std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty());
    let api_key_for_model = api_key.clone();
    let api_key_for_commit = api_key.clone();
    let api_key_for_slug = api_key.clone();

    let goal_slug_generator: Arc<dyn GoalSlugGenerator> = if let (Some(key), Some(model_name)) =
        (api_key_for_slug, small_model.clone())
    {
        let slug_prompt = "Generate a short slug for this goal. Return only lowercase letters, numbers, and hyphens. No spaces, punctuation, or extra text.";
        Arc::new(OpenAIGoalSlugModel::new(
            key,
            model_name,
            slug_prompt.to_string(),
        ))
    } else {
        Arc::new(MockGoalSlugModel)
    };

    // Determine data directory
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("rx_data"));
    let db_path = data_dir.join("rx_state.db");

    // Initialize State
    let state_store = Arc::new(SqliteStateStore::new(db_path)?);

    if list_goals {
        let goals = state_store.list_goals().await?;
        for (goal_id, timestamp) in goals {
            println!("{} - {}", timestamp, goal_id);
        }
        return Ok(());
    }

    if goal_id_to_resume.is_none() && goal_parts.is_empty() {
        eprintln!("Usage: rx <goal> [--max-iterations N] [--resume <goal_id>] [--debug-log <path>] [--list] [--tool-verbose] [--model <name>] [--small-model <name>] [--agent <name>]");
        std::process::exit(1);
    }

    let goal_id = if let Some(goal_id) = goal_id_to_resume.clone() {
        let events: Vec<Event> = state_store.load(&goal_id).await?;
        if events.is_empty() {
            eprintln!("No events found for goal ID: {}", goal_id);
            std::process::exit(1);
        }
        println!("Resuming Goal ID: {}", goal_id);
        goal_id
    } else {
        let goal = goal_parts.join(" ");
        let timestamp_id = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let goal_slug = match goal_slug_generator.goal_slug(&goal).await {
            Ok(slug) => slug,
            Err(error) => {
                eprintln!(
                    "Warning: failed to generate goal slug from model: {}. Falling back to deterministic slug.",
                    error
                );
                crate::model::sanitize_goal_slug(&goal)
            }
        };
        let new_goal_id = format!("{}-{}", timestamp_id, goal_slug);
        println!("New Goal ID: {}", new_goal_id);
        state_store
            .append_event(
                &new_goal_id,
                Event::new("goal", serde_json::json!({ "goal": goal })),
            )
            .await?;
        new_goal_id
    };

    // Initialize tools
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(BashTool));
    registry.register(Arc::new(ExecTool));
    registry.register(Arc::new(ExecCaptureTool));
    registry.register(Arc::new(ExecStatusTool));
    registry.register(Arc::new(ExecWithInputTool));
    registry.register(Arc::new(WhichCommandTool));
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(ListDirTool));
    registry.register(Arc::new(StatFileTool));
    registry.register(Arc::new(DoneTool));

    // Load prompt
    let prompt_path = "LOOP_PROMPT.md";
    let system_prompt = fs::read_to_string(prompt_path)
        .await
        .context(format!("Failed to read {}", prompt_path))?;

    let debug_log_path = debug_log_template
        .as_ref()
        .map(|template| expand_debug_log_path(template, &goal_id));
    let debug_log_display = debug_log_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "disabled".to_string());
    let resume_display = goal_id_to_resume.as_deref().unwrap_or("none").to_string();
    let small_model_display = small_model.clone().unwrap_or_else(|| "none".to_string());

    if small_model_from_legacy_config {
        eprintln!("Warning: config key auto_commit_model is deprecated; use small_model instead.");
    }

    eprintln!("Effective config:");
    eprintln!("  source: {}", config_source);
    eprintln!("  max_iterations: {}", max_iterations);
    eprintln!("  auto_commit: {}", auto_commit);
    eprintln!("  small_model: {}", small_model_display);
    eprintln!("  list: {}", list_goals);
    eprintln!("  resume_goal_id: {}", resume_display);
    eprintln!("  debug_log: {}", debug_log_display);
    eprintln!("  tool_verbose: {}", tool_verbose);
    eprintln!("  model: {}", model_name);
    eprintln!(
        "  api_key_present: {}",
        if api_key.is_some() { "true" } else { "false" }
    );

    let model: Arc<dyn Model> = if let Some(key) = api_key_for_model {
        Arc::new(OpenAIModel::new(key, model_name, &registry, system_prompt))
    } else {
        println!("Warning: OPENAI_API_KEY not set. Using MockModel for testing.");
        Arc::new(MockModel)
    };

    let commit_message_generator: Option<Arc<dyn CommitMessageGenerator>> = if auto_commit {
        if let Some(commit_model) = small_model.take() {
            if let Some(key) = api_key_for_commit {
                let commit_prompt = "Generate a concise git commit message (max 50 chars) in imperative mood. Respond with only the message.";
                Some(Arc::new(OpenAICommitMessageModel::new(
                    key,
                    commit_model,
                    commit_prompt.to_string(),
                )))
            } else {
                println!(
                    "Warning: small_model configured but OPENAI_API_KEY not set. Falling back to default commit messages."
                );
                Some(Arc::new(MockCommitMessageModel))
            }
        } else {
            None
        }
    } else {
        None
    };

    let debug_logger = if let Some(path) = debug_log_path {
        Some(Arc::new(DebugLogger::new(path).await?))
    } else {
        None
    };

    let kernel = Kernel::new(
        goal_id.clone(),
        model,
        state_store.clone(),
        registry,
        max_iterations,
        auto_commit,
        commit_message_generator,
        debug_logger,
        tool_verbose,
    );

    if let Err(e) = kernel.run().await {
        eprintln!("Kernel error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
