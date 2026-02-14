use crate::event::Event;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Message(String),
    ToolCall(ToolCall),
}

#[async_trait]
pub trait Model: Send + Sync {
    async fn next_action(&self, history: &[Event]) -> Result<Action>;
}

pub struct MockModel {
    _system_prompt: String,
    goal: String,
    goal_slug: String,
}

impl MockModel {
    pub fn new(system_prompt: String, goal: String, goal_slug: String) -> Self {
        Self {
            _system_prompt: system_prompt,
            goal,
            goal_slug,
        }
    }
}

#[async_trait]
impl Model for MockModel {
    async fn next_action(&self, history: &[Event]) -> Result<Action> {
        let tool_outputs = history
            .iter()
            .filter(|event| event.r#type == "tool_output")
            .count();
        let call_id = format!("call-{}", tool_outputs + 1);

        let action = match tool_outputs {
            0 => Action::ToolCall(ToolCall {
                id: call_id,
                name: "write_file".to_string(),
                arguments: json!({
                    "path": "hello.txt",
                    "content": format!("Goal '{}': hello from rx minimal", self.goal),
                    "mode": "overwrite"
                }),
            }),
            1 => Action::ToolCall(ToolCall {
                id: call_id,
                name: "list_dir".to_string(),
                arguments: json!({ "path": "." }),
            }),
            2 => Action::ToolCall(ToolCall {
                id: call_id,
                name: "done".to_string(),
                arguments: json!({
                    "reason": "mock completion",
                    "details": {
                        "goal": self.goal,
                        "goal_slug": self.goal_slug
                    }
                }),
            }),
            _ => Action::Message("No further actions".to_string()),
        };

        Ok(action)
    }
}
