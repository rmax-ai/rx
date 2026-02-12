use crate::event::Event;
use crate::model::{Model, Action};
use crate::state::StateStore;
use crate::tool::ToolRegistry;
use anyhow::Result;
use std::sync::Arc;

pub struct Kernel {
    goal_id: String,
    model: Arc<dyn Model>,
    state_store: Arc<dyn StateStore>,
    tool_registry: ToolRegistry,
    max_iterations: usize,
    auto_commit: bool,
}

impl Kernel {
    pub fn new(
        goal_id: String,
        model: Arc<dyn Model>,
        state_store: Arc<dyn StateStore>,
        tool_registry: ToolRegistry,
        max_iterations: usize,
        auto_commit: bool,
    ) -> Self {
        Self {
            goal_id,
            model,
            state_store,
            tool_registry,
            max_iterations,
            auto_commit,
        }
    }
    
    pub async fn run(&self) -> Result<()> {
        let mut iteration = 0;
        
        loop {
            if iteration >= self.max_iterations {
                println!("Max iterations reached");
                self.state_store.append_event(&self.goal_id, Event::new("termination", serde_json::json!({ "reason": "max_iterations" }))).await?;
                break;
            }
            iteration += 1;
            println!("Iteration {}", iteration);

            let history = self.state_store.load(&self.goal_id).await?;
            let action = self.model.next_action(&history).await?;

            match action {
                Action::Message(content) => {
                    println!("Model Message: {}", content);
                    self.state_store.append_event(&self.goal_id, Event::new("action", serde_json::json!(Action::Message(content)))).await?;
                    // Messages don't advance state much, but we record them.
                    // If model keeps sending messages without tool calls, we might loop.
                    // But maybe it's thinking.
                },
                Action::ToolCall(tool_call) => {
                    println!("Tool Call: {} (id={})", tool_call.name, tool_call.id);
                    self.state_store.append_event(&self.goal_id, Event::new("action", serde_json::json!(Action::ToolCall(tool_call.clone())))).await?;
                    
                    let tool = self.tool_registry.get(&tool_call.name);
                    let output = if let Some(tool) = tool {
                        match tool.execute(tool_call.arguments.clone()).await {
                            Ok(output) => output,
                            Err(e) => serde_json::json!({ "error": e.to_string() }),
                        }
                    } else {
                        serde_json::json!({ "error": format!("Tool {} not found", tool_call.name) })
                    };
                    
                    self.state_store.append_event(&self.goal_id, Event::new("tool_output", serde_json::json!({
                        "tool_call_id": tool_call.id,
                        "output": output
                    }))).await?;

                    if self.auto_commit {
                        self.perform_commit(&iteration.to_string()).await.ok();
                    }

                    if tool_call.name == "done" {
                         println!("Goal achieved or stopped via done tool.");
                         self.state_store.append_event(&self.goal_id, Event::new("termination", serde_json::json!({ "reason": "done", "details": output }))).await?;
                         break;
                    }
                }
            }
        }
        Ok(())
    }

    async fn perform_commit(&self, iteration: &str) -> Result<()> {
        if let Some(exec_tool) = self.tool_registry.get("exec") {
             exec_tool.execute(serde_json::json!({
                 "command": "git",
                 "args": ["add", "."]
             })).await.ok();
             
             exec_tool.execute(serde_json::json!({
                 "command": "git",
                 "args": ["commit", "-m", format!("rx: iteration {}", iteration)]
             })).await.ok();
        }
        Ok(())
    }
}
