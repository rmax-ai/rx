use crate::kernel::Kernel;
use crate::model::OpenAIModel;
use crate::state::{StateStore, InMemoryStateStore};
use crate::tool::ToolRegistry;
use crate::tools::{done::DoneTool, exec::ExecTool, fs::{ReadFileTool, WriteFileTool, ListDirTool}};
use crate::event::Event;
use std::sync::Arc;
use anyhow::{Result, Context};
use tokio::fs;

pub mod kernel;
pub mod model;
pub mod tool;
pub mod state;
pub mod event;
pub mod tools;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: rx <goal>");
        std::process::exit(1);
    }
    // Combine all args after the first one as the goal
    let goal = args[1..].join(" ");
    
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
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let model_name = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
    
    let model = Arc::new(OpenAIModel::new(api_key, model_name, &registry, system_prompt));

    // Initialize State
    let state_store = Arc::new(InMemoryStateStore::new());

    // Initialize Kernel
    let kernel = Kernel::new(goal_id.clone(), model, state_store.clone(), registry);

    // Initial event: Goal
    state_store.append_event(&goal_id, Event::new("goal", serde_json::json!({ "goal": goal }))).await?;

    // Run
    if let Err(e) = kernel.run().await {
        eprintln!("Kernel error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
