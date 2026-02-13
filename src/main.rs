use crate::kernel::Kernel;
use crate::model::{Model, OpenAIModel, MockModel};
use crate::state::{StateStore, InMemoryStateStore, SqliteStateStore};
use crate::tool::ToolRegistry;
use crate::tools::{done::DoneTool, exec::ExecTool, fs::{ReadFileTool, WriteFileTool, ListDirTool}};
use crate::event::Event;
use std::sync::Arc;
use anyhow::{Result, Context};
use tokio::fs;
use dirs;
use std::path::PathBuf;

pub mod kernel;
pub mod model;
pub mod tool;
pub mod state;
pub mod event;
pub mod tools;

#[tokio::main]
async fn main() -> Result<()> {
    let mut max_iterations = 50;
    let mut auto_commit = false;
    let mut goal_parts = Vec::new();
    let mut args_iter = std::env::args().skip(1);

    while let Some(arg) = args_iter.next() {
        if arg == "--max-iterations" {
            if let Some(val) = args_iter.next() {
                max_iterations = val.parse().unwrap_or(50);
            }
        } else if arg == "--auto-commit" {
            auto_commit = true;
        } else {
            goal_parts.push(arg);
        }
    }

    if goal_parts.is_empty() {
        eprintln!("Usage: rx <goal> [--max-iterations N]");
        std::process::exit(1);
    }

    let goal = goal_parts.join(" ");

    // Generate simple ID
    let goal_id = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    println!("Goal ID: {}", goal_id);

    // Initialize tools
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(ExecTool));
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(ListDirTool));
    registry.register(Arc::new(DoneTool));

    // Load prompt
    let prompt_path = "LOOP_PROMPT.md";
    let system_prompt = fs::read_to_string(prompt_path).await
        .context(format!("Failed to read {}", prompt_path))?;

    // Initialize Model
    let api_key = std::env::var("OPENAI_API_KEY").ok().filter(|k| !k.is_empty());
    let model_name = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

    let model: Arc<dyn Model> = if let Some(key) = api_key {
        Arc::new(OpenAIModel::new(key, model_name, &registry, system_prompt))
    } else {
        println!("Warning: OPENAI_API_KEY not set. Using MockModel for testing.");
        Arc::new(MockModel)
    };

    // Determine data directory
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("rx_data"));
    let db_path = data_dir.join("rx_state.db");
    
    // Initialize State
    let state_store = Arc::new(SqliteStateStore::new(db_path)?);

    // Initialize Kernel
    let kernel = Kernel::new(goal_id.clone(), model, state_store.clone(), registry, max_iterations, auto_commit);

    // Initial event: Goal
    state_store.append_event(&goal_id, Event::new("goal", serde_json::json!({ "goal": goal }))).await?;

    // Run
    if let Err(e) = kernel.run().await {
        eprintln!("Kernel error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
