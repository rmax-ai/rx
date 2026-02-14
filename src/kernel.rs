use crate::event::Event;
use crate::model::{Action, Model, ToolCall};
use crate::state::StateStore;
use crate::tool::ToolRegistry;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct Kernel {
    goal_id: String,
    model: Arc<dyn Model>,
    state_store: Arc<dyn StateStore>,
    tool_registry: ToolRegistry,
    max_iterations: usize,
}

impl Kernel {
    pub fn new(
        goal_id: String,
        model: Arc<dyn Model>,
        state_store: Arc<dyn StateStore>,
        tool_registry: ToolRegistry,
        max_iterations: usize,
    ) -> Self {
        Self {
            goal_id,
            model,
            state_store,
            tool_registry,
            max_iterations,
        }
    }

    pub async fn run(&self) -> Result<()> {
        println!("Starting goal {}", self.goal_id);

        for iteration in 1..=self.max_iterations {
            println!("Iteration {}/{}", iteration, self.max_iterations);
            let history = self.state_store.load().await?;
            let action = self.model.next_action(&history).await?;

            self.state_store
                .append_event(Event::new("action", serde_json::json!(action.clone())))
                .await?;

            match action {
                Action::Message(message) => {
                    println!("model message: {}", message);
                }
                Action::ToolCall(tool_call) => {
                    println!("tool call: {} [{}]", tool_call.name, tool_call.id);
                    let output = self.execute_tool(&tool_call).await;

                    self.state_store
                        .append_event(Event::new(
                            "tool_output",
                            json!({
                                "tool_call_id": tool_call.id,
                                "name": tool_call.name,
                                "output": output,
                            }),
                        ))
                        .await?;

                    if tool_call.name == "done" {
                        println!("termination requested by done tool");
                        self.state_store
                            .append_event(Event::new(
                                "termination",
                                json!({
                                    "reason": "done",
                                    "iteration": iteration,
                                    "details": output,
                                }),
                            ))
                            .await?;
                        return Ok(());
                    }
                }
            }
        }

        println!("Max iterations reached ({})", self.max_iterations);
        self.state_store
            .append_event(Event::new(
                "termination",
                json!({
                    "reason": "max_iterations",
                    "iterations": self.max_iterations,
                }),
            ))
            .await?;

        Ok(())
    }

    async fn execute_tool(&self, tool_call: &ToolCall) -> Value {
        if let Some(tool) = self.tool_registry.get(&tool_call.name) {
            match tool.execute(tool_call.arguments.clone()).await {
                Ok(output) => output,
                Err(error) => json!({ "error": error.to_string() }),
            }
        } else {
            json!({ "error": format!("tool {} not registered", tool_call.name) })
        }
    }
}
