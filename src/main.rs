use crate::config_loader::load_config;
use crate::debug_logger::DebugLogger;
use crate::event::Event;
use crate::kernel::Kernel;
use crate::model::{CommitMessageGenerator, MockCommitMessageModel, MockModel, Model, OpenAICommitMessageModel, OpenAIModel};
use crate::sqlite_state::SqliteStateStore;
use crate::state::StateStore;
use crate::tool::ToolRegistry;
use crate::tools::{
    done::DoneTool,
    exec::ExecTool,
    fs::{ListDirTool, ReadFileTool, WriteFileTool},
};
use anyhow::{Context, Result};
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

#[tokio::main]
async fn main() -> Result<()> {
    // Load Config
    let config_path_source = resolve_config_path();
    let config_path = config_path_source.path().to_path_buf();
    let config_description = format!(
        "{} ({})",
        config_path_source.description(),
        config_path.display()
    );
    let (config, config_source) = match load_config(&config_path) {
        Ok(config) => (config, format!("loaded from {}", config_description)),
        Err(err) => {
            eprintln!(
                "Warning: failed to load config from {}: {}. Using defaults.",
                config_path.display(),
                err
            );
            (
                crate::config_loader::CliDefaults::default(),
                format!("defaults (config load failed at {})", config_description),
            )
        }
    };

    let mut max_iterations = config.max_iterations.unwrap_or(50);
    let mut auto_commit = config.auto_commit.unwrap_or(false);
    let mut auto_commit_model = config.auto_commit_model.clone();
    let mut goal_id_to_resume = None;
    let mut debug_log_path: Option<PathBuf> = config.debug_log.map(PathBuf::from);
    let mut goal_parts = Vec::new();
    let mut args_iter = std::env::args().skip(1);
    let mut list_goals = config.list.unwrap_or(false);
    let mut tool_verbose = config.tool_verbose.unwrap_or(false);

    // New: Check config for model name
    let mut model_name = config
        .model_name
        .unwrap_or_else(|| "gpt-4o".to_string());

    while let Some(arg) = args_iter.next() {
        if arg == "--max-iterations" {
            if let Some(val) = args_iter.next() {
                max_iterations = val.parse().unwrap_or(max_iterations);
            }
        } else if arg == "--auto-commit" {
            auto_commit = true;
        } else if arg == "--resume" {
            if let Some(goal_id) = args_iter.next() {
                goal_id_to_resume = Some(goal_id);
            } else {
                eprintln!("--resume flag requires a goal ID.");
                std::process::exit(1);
            }
        } else if arg == "--debug-log" {
            if let Some(path) = args_iter.next() {
                debug_log_path = Some(PathBuf::from(path));
            } else {
                eprintln!("--debug-log flag requires a file path.");
                std::process::exit(1);
            }
        } else if arg == "--list" {
            list_goals = true;
        } else if arg == "--tool-verbose" {
            tool_verbose = true;
        } else {
            goal_parts.push(arg);
        }
    }

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
        eprintln!("Usage: rx <goal> [--max-iterations N] [--resume <goal_id>] [--debug-log <path>] [--list] [--tool-verbose]");
        std::process::exit(1);
    }

    let goal_id = if let Some(goal_id) = goal_id_to_resume.clone() {
        // Check for existing events for the given goal_id
        let events: Vec<Event> = state_store.load(&goal_id).await?;
        if events.is_empty() {
            eprintln!("No events found for goal ID: {}", goal_id);
            std::process::exit(1);
        }
        println!("Resuming Goal ID: {}", goal_id);
        goal_id
    } else {
        // New Goal
        let goal = goal_parts.join(" ");
        let new_goal_id = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
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
    registry.register(Arc::new(ExecTool));
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(ListDirTool));
    registry.register(Arc::new(DoneTool));

    // Load prompt
    let prompt_path = "LOOP_PROMPT.md";
    let system_prompt = fs::read_to_string(prompt_path)
        .await
        .context(format!("Failed to read {}", prompt_path))?;

    // Initialize Model
    let api_key = std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty());
    let api_key_for_model = api_key.clone();
    let api_key_for_commit = api_key.clone();

    // Set model name preference based on config or env variable
    if let Ok(env_model_name) = std::env::var("OPENAI_MODEL") {
        model_name = env_model_name;
    }

    let debug_log_display = debug_log_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "disabled".to_string());
    let resume_display = goal_id_to_resume.as_deref().unwrap_or("none").to_string();
    let auto_commit_display = auto_commit_model.clone().unwrap_or_else(|| "none".to_string());

    eprintln!("Effective config:");
    eprintln!("  source: {}", config_source);
    eprintln!("  max_iterations: {}", max_iterations);
    eprintln!("  auto_commit: {}", auto_commit);
    eprintln!("  auto_commit_model: {}", auto_commit_display);
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
        if let Some(commit_model) = auto_commit_model.take() {
            if let Some(key) = api_key_for_commit {
                let commit_prompt = "Generate a concise git commit message (max 50 chars) in imperative mood. Respond with only the message.";
                Some(Arc::new(OpenAICommitMessageModel::new(
                    key,
                    commit_model,
                    commit_prompt.to_string(),
                )))
            } else {
                println!(
                    "Warning: auto_commit_model configured but OPENAI_API_KEY not set. Falling back to default commit messages."
                );
                Some(Arc::new(MockCommitMessageModel))
            }
        } else {
            None
        }
    } else {
        None
    };

    // Initialize Kernel
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

    // Run
    if let Err(e) = kernel.run().await {
        eprintln!("Kernel error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
