use crate::debug_logger::DebugLogger;
use crate::event::Event;
use crate::model::{Action, CommitMessageGenerator, Model};
use crate::state::StateStore;
use crate::tool::ToolRegistry;
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;

pub struct Kernel {
    goal_id: String,
    model: Arc<dyn Model>,
    state_store: Arc<dyn StateStore>,
    tool_registry: ToolRegistry,
    max_iterations: usize,
    auto_commit: bool,
    commit_message_generator: Option<Arc<dyn CommitMessageGenerator>>,
    debug_logger: Option<Arc<DebugLogger>>,
    tool_verbose: bool,
}

impl Kernel {
    pub fn new(
        goal_id: String,
        model: Arc<dyn Model>,
        state_store: Arc<dyn StateStore>,
        tool_registry: ToolRegistry,
        max_iterations: usize,
        auto_commit: bool,
        commit_message_generator: Option<Arc<dyn CommitMessageGenerator>>,
        debug_logger: Option<Arc<DebugLogger>>,
        tool_verbose: bool,
    ) -> Self {
        Self {
            goal_id,
            model,
            state_store,
            tool_registry,
            max_iterations,
            auto_commit,
            commit_message_generator,
            debug_logger,
            tool_verbose,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut iteration = 0;

        loop {
            if iteration >= self.max_iterations {
                println!("Max iterations reached");
                self.append_termination("max_iterations", json!({ "reason": "max_iterations" }))
                    .await?;
                break;
            }
            iteration += 1;
            println!("Iteration {}", iteration);

            let history = self.state_store.load(&self.goal_id).await?;
            let action = self.model.next_action(&history).await?;

            self.log_debug(json!({
                "event": "action_decision",
                "iteration": iteration,
                "action": format!("{:?}", action),
            }))
            .await;

            match action {
                Action::Message(content) => {
                    println!("Model Message: {}", content);
                    self.append_action(Action::Message(content.clone())).await?;
                }
                Action::ToolCall(tool_call) => {
                    println!("Tool Call: {} (id={})", tool_call.name, tool_call.id);
                    if self.tool_verbose {
                        println!("Tool Input ({}): {}", tool_call.name, tool_call.arguments);
                    }
                    self.append_action(Action::ToolCall(tool_call.clone()))
                        .await?;

                    let output = self.execute_tool(&tool_call).await;
                    if self.tool_verbose {
                        println!("Tool Output ({}): {}", tool_call.name, output);
                    }
                    self.append_tool_output(&tool_call, &output).await?;

                    if self.auto_commit {
                        self.perform_commit().await.ok();
                    }

                    if tool_call.name == "done" {
                        println!("Goal achieved or stopped via done tool.");
                        self.append_termination(
                            "done",
                            json!({ "reason": "done", "details": output }),
                        )
                        .await?;
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    async fn append_action(&self, action: Action) -> Result<()> {
        self.state_store
            .append_event(
                &self.goal_id,
                Event::new("action", serde_json::json!(action)),
            )
            .await
    }

    async fn append_tool_output(
        &self,
        tool_call: &crate::model::ToolCall,
        output: &serde_json::Value,
    ) -> Result<()> {
        self.state_store
            .append_event(
                &self.goal_id,
                Event::new(
                    "tool_output",
                    serde_json::json!({
                        "tool_call_id": tool_call.id,
                        "output": output
                    }),
                ),
            )
            .await
    }

    async fn append_termination(&self, reason: &str, details: serde_json::Value) -> Result<()> {
        self.state_store
            .append_event(
                &self.goal_id,
                Event::new(
                    "termination",
                    serde_json::json!({
                        "reason": reason,
                        "details": details,
                    }),
                ),
            )
            .await
    }

    async fn execute_tool(&self, tool_call: &crate::model::ToolCall) -> serde_json::Value {
        if let Some(tool) = self.tool_registry.get(&tool_call.name) {
            match tool.execute(tool_call.arguments.clone()).await {
                Ok(output) => output,
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            }
        } else {
            serde_json::json!({ "error": format!("Tool {} not found", tool_call.name) })
        }
    }

    async fn log_debug(&self, entry: serde_json::Value) {
        if let Some(logger) = &self.debug_logger {
            let _ = logger.log(&entry).await;
        }
    }

    async fn perform_commit(&self) -> Result<()> {
        let exec_tool = match self.tool_registry.get("exec") {
            Some(tool) => tool,
            None => return Ok(()),
        };

        exec_tool
            .execute(serde_json::json!({
                "command": "git",
                "args": ["add", "."]
            }))
            .await
            .ok();

        let diff_output = exec_tool
            .execute(serde_json::json!({
                "command": "git",
                "args": ["diff", "--cached"]
            }))
            .await
            .ok();

        let diff_text = diff_output
            .as_ref()
            .and_then(|value| value.get("stdout"))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if diff_text.is_empty() {
            return Ok(());
        }

        let message = if let Some(generator) = &self.commit_message_generator {
            match generator.commit_message(&diff_text).await {
                Ok(message) if !message.trim().is_empty() => message,
                _ => "rx: update".to_string(),
            }
        } else {
            "rx: update".to_string()
        };

        exec_tool
            .execute(serde_json::json!({
                "command": "git",
                "args": ["commit", "-m", message]
            }))
            .await
            .ok();

        Ok(())
    }
}
