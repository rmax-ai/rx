mod event;
mod kernel;
mod model;
mod runtime_hooks;
mod state;
mod tool;
mod tools;
mod utils;

use crate::event::Event;
use crate::kernel::Kernel;
use crate::model::{MockModel, Model, OpenAIModel};
use crate::runtime_hooks::{
    AutoCommitHook, DebugJsonlHook, EventHook, HeuristicCommitMessageGenerator, HookedStateStore,
    ToolVerboseHook,
};
use crate::state::{InMemoryStateStore, StateStore};
use crate::tool::ToolRegistry;
use crate::tools::done::DoneTool;
use crate::tools::exec::ExecTool;
use crate::tools::fs::{
    AppendFileTool, ApplyUnifiedPatchTool, CreateFileTool, ListDirTool, ReadFileTool,
    ReplaceInFileTool, WriteFileTool,
};
use crate::utils::sanitize_goal_slug;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

struct CliArgs {
    goal: String,
    max_iterations: usize,
    model_name: Option<String>,
    auto_commit: bool,
    tool_verbose: bool,
    debug_log_path: Option<PathBuf>,
}

fn parse_cli_args() -> CliArgs {
    let mut args = std::env::args().skip(1);
    let mut max_iterations = 50;
    let mut model_name = None;
    let mut auto_commit = false;
    let mut tool_verbose = false;
    let mut debug_log_path = None;
    let mut goal_parts = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-iterations" => {
                if let Some(value) = args.next() {
                    if let Ok(parsed) = value.parse::<usize>() {
                        max_iterations = parsed;
                    } else {
                        eprintln!("Warning: ignoring invalid max iterations '{}'.", value);
                    }
                } else {
                    eprintln!(
                        "Warning: --max-iterations requires a value. Using default {}.",
                        max_iterations
                    );
                }
            }
            "--model" => {
                if let Some(value) = args.next() {
                    if value.trim().is_empty() {
                        eprintln!("Warning: ignoring empty --model value.");
                    } else {
                        model_name = Some(value);
                    }
                } else {
                    eprintln!("Warning: --model requires a value.");
                }
            }
            "--auto-commit" => auto_commit = true,
            "--tool-verbose" => tool_verbose = true,
            "--debug-log" => {
                if let Some(value) = args.next() {
                    debug_log_path = Some(PathBuf::from(value));
                } else {
                    eprintln!("Warning: --debug-log requires a file path.");
                }
            }
            other => goal_parts.push(other.to_string()),
        }
    }

    let goal = goal_parts.join(" ").trim().to_string();
    if goal.is_empty() {
        eprintln!(
            "Usage: rx [--max-iterations N] [--model NAME] [--auto-commit] [--tool-verbose] [--debug-log PATH] <goal>"
        );
        std::process::exit(1);
    }

    CliArgs {
        goal,
        max_iterations,
        model_name,
        auto_commit,
        tool_verbose,
        debug_log_path,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let CliArgs {
        goal,
        max_iterations,
        model_name,
        auto_commit,
        tool_verbose,
        debug_log_path,
    } = parse_cli_args();
    let goal_slug = sanitize_goal_slug(&goal);
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let goal_id = format!("{}-{}", timestamp, goal_slug);

    let system_prompt = fs::read_to_string("LOOP_PROMPT.md")
        .await
        .context("failed to read LOOP_PROMPT.md")?;

    let base_state_store: Arc<dyn StateStore> = Arc::new(InMemoryStateStore::new(&goal_id).await?);
    let mut hooks: Vec<Arc<dyn EventHook>> = Vec::new();

    if let Some(path) = debug_log_path {
        hooks.push(Arc::new(DebugJsonlHook::new(&path).await?));
    }
    if tool_verbose {
        hooks.push(Arc::new(ToolVerboseHook));
    }
    if auto_commit {
        let generator = Arc::new(HeuristicCommitMessageGenerator);
        hooks.push(Arc::new(AutoCommitHook::new(generator)));
    }

    let state_store: Arc<dyn StateStore> = if hooks.is_empty() {
        Arc::clone(&base_state_store)
    } else {
        Arc::new(HookedStateStore::new(Arc::clone(&base_state_store), hooks))
    };

    state_store
        .append_event(Event::new(
            "goal",
            json!({
                "goal": goal.clone(),
                "goal_id": goal_id.clone()
            }),
        ))
        .await?;

    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ExecTool));
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(CreateFileTool));
    registry.register(Arc::new(AppendFileTool));
    registry.register(Arc::new(ReplaceInFileTool));
    registry.register(Arc::new(ApplyUnifiedPatchTool));
    registry.register(Arc::new(ListDirTool));
    registry.register(Arc::new(DoneTool));

    let resolved_model_name = model_name
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .unwrap_or_else(|| "gpt-4o".to_string());

    let model: Arc<dyn Model> = if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        if api_key.trim().is_empty() {
            eprintln!("Warning: OPENAI_API_KEY is empty. Using MockModel.");
            Arc::new(MockModel::new(system_prompt, goal, goal_slug))
        } else {
            Arc::new(OpenAIModel::new(
                api_key,
                resolved_model_name,
                &registry,
                system_prompt,
            ))
        }
    } else {
        eprintln!("Warning: OPENAI_API_KEY not set. Using MockModel.");
        Arc::new(MockModel::new(system_prompt, goal, goal_slug))
    };

    let kernel = Kernel::new(
        goal_id.clone(),
        model,
        Arc::clone(&state_store),
        registry,
        max_iterations,
    );

    kernel.run().await?;
    Ok(())
}
