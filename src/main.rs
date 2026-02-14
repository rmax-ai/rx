mod event;
mod kernel;
mod model;
mod state;
mod tool;
mod tools;
mod utils;

use crate::event::Event;
use crate::kernel::Kernel;
use crate::model::{MockModel, Model};
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
use std::sync::Arc;
use tokio::fs;

struct CliArgs {
    goal: String,
    max_iterations: usize,
}

fn parse_cli_args() -> CliArgs {
    let mut args = std::env::args().skip(1);
    let mut max_iterations = 50;
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
            other => goal_parts.push(other.to_string()),
        }
    }

    let goal = goal_parts.join(" ").trim().to_string();
    if goal.is_empty() {
        eprintln!("Usage: rx [--max-iterations N] <goal>");
        std::process::exit(1);
    }

    CliArgs {
        goal,
        max_iterations,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let CliArgs {
        goal,
        max_iterations,
    } = parse_cli_args();
    let goal_slug = sanitize_goal_slug(&goal);
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let goal_id = format!("{}-{}", timestamp, goal_slug);

    let system_prompt = fs::read_to_string("LOOP_PROMPT.md")
        .await
        .context("failed to read LOOP_PROMPT.md")?;

    let state_store: Arc<dyn StateStore> = Arc::new(InMemoryStateStore::new(&goal_id).await?);
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

    let model: Arc<dyn Model> = Arc::new(MockModel::new(system_prompt, goal, goal_slug));

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
