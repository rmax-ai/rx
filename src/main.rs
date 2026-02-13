use crate::kernel::Kernel;
use crate::model::{Model, OpenAIModel, MockModel};
use crate::state::{StateStore};
use crate::tool::ToolRegistry;
use crate::tools::{done::DoneTool, exec::ExecTool, fs::{ReadFileTool, WriteFileTool, ListDirTool}};
use crate::event::Event;
use crate::sqlite_state::SqliteStateStore;
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
pub mod sqlite_state;

#[tokio::main]
async fn main() -> Result<()> {
    let mut max_iterations = 50;
    let mut auto_commit = false;
    let mut goal_id_to_resume = None;
    let mut goal_parts = Vec::new();
    let mut args_iter = std::env::args().skip(1);
    let mut list_goals = false;

    while let Some(arg) = args_iter.next() {
        if arg == "--max-iterations" {
            if let Some(val) = args_iter.next() {
                max_iterations = val.parse().unwrap_or(50);
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
        } else if arg == "--list" {
            list_goals = true;
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
        eprintln!("Usage: rx <goal> [--max-iterations N] [--resume <goal_id>] [--list]");
        std::process::exit(1);
    }

    let goal_id = if let Some(goal_id) = goal_id_to_resume {
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
        state_store.append_event(&new_goal_id, Event::new("goal", serde_json::json!({ "goal": goal }))).await?;
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

    // Initialize Kernel
    let kernel = Kernel::new(goal_id.clone(), model, state_store.clone(), registry, max_iterations, auto_commit);

    // Run
    if let Err(e) = kernel.run().await {
        eprintln!("Kernel error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
