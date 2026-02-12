use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;
use async_trait::async_trait;
use crate::event::Event;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
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
