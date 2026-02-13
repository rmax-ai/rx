use crate::event::Event;
use crate::tool::ToolRegistry;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
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

pub struct OpenAIModel {
    client: Client,
    api_key: String,
    model_name: String,
    tools: Value,
    system_prompt: String,
}

impl OpenAIModel {
    pub fn new(
        api_key: String,
        model_name: String,
        registry: &ToolRegistry,
        system_prompt: String,
    ) -> Self {
        let tools_json: Vec<Value> = registry
            .list()
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters()
                    }
                })
            })
            .collect();

        Self {
            client: Client::new(),
            api_key,
            model_name,
            tools: json!(tools_json),
            system_prompt,
        }
    }

    fn events_to_messages(&self, history: &[Event]) -> Vec<Value> {
        let mut messages = vec![json!({ "role": "system", "content": self.system_prompt })];

        for event in history {
            match event.r#type.as_str() {
                "goal" => {
                    // Initial user goal
                    if let Some(content) = event.payload.get("goal").and_then(|v| v.as_str()) {
                        messages.push(json!({ "role": "user", "content": content }));
                    }
                }
                "action" => {
                    // Assistant action
                    if let Ok(action) = serde_json::from_value::<Action>(event.payload.clone()) {
                        match action {
                            Action::Message(content) => {
                                messages.push(json!({ "role": "assistant", "content": content }));
                            }
                            Action::ToolCall(tool_call) => {
                                messages.push(json!({
                                    "role": "assistant",
                                    "content": null,
                                    "tool_calls": [{
                                        "id": tool_call.id,
                                        "type": "function",
                                        "function": {
                                            "name": tool_call.name,
                                            "arguments": tool_call.arguments.to_string()
                                        }
                                    }]
                                }));
                            }
                        }
                    }
                }
                "tool_output" => {
                    // Tool result
                    // Payload should have tool_call_id and output
                    let tool_call_id = event
                        .payload
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let output = event.payload.get("output").cloned().unwrap_or(Value::Null);
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tool_call_id,
                        "content": output.to_string()
                    }));
                }
                _ => {}
            }
        }
        messages
    }
}

#[async_trait]
impl Model for OpenAIModel {
    async fn next_action(&self, history: &[Event]) -> Result<Action> {
        let messages = self.events_to_messages(history);

        let request_body = json!({
            "model": self.model_name,
            "messages": messages,
            "tools": self.tools,
            "tool_choice": "auto"
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let response_body: Value = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        // Parse choice
        let choice = response_body["choices"]
            .get(0)
            .context("No choices in response")?;
        let message = &choice["message"];

        if let Some(tool_calls) = message["tool_calls"].as_array() {
            if let Some(first_call) = tool_calls.first() {
                let id = first_call["id"].as_str().unwrap_or_default().to_string();
                let func = &first_call["function"];
                let name = func["name"].as_str().unwrap_or_default().to_string();
                let args_str = func["arguments"].as_str().unwrap_or("{}");
                let args_val: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                return Ok(Action::ToolCall(ToolCall {
                    id,
                    name,
                    arguments: args_val,
                }));
            }
        }

        let content = message["content"].as_str().unwrap_or("").to_string();
        Ok(Action::Message(content))
    }
}

pub struct MockModel;

#[async_trait]
impl Model for MockModel {
    async fn next_action(&self, history: &[Event]) -> Result<Action> {
        let tool_outputs = history.iter().filter(|e| e.r#type == "tool_output").count();

        match tool_outputs {
            0 => Ok(Action::ToolCall(ToolCall {
                id: "call_1".to_string(),
                name: "write_file".to_string(),
                arguments: json!({
                    "path": "hello.txt",
                    "content": "Hello world",
                    "mode": "create"
                }),
            })),
            1 => Ok(Action::ToolCall(ToolCall {
                id: "call_2".to_string(),
                name: "exec".to_string(),
                arguments: json!({
                    "command": "ls",
                    "args": ["-F"]
                }),
            })),
            2 => Ok(Action::ToolCall(ToolCall {
                id: "call_3".to_string(),
                name: "done".to_string(),
                arguments: json!({
                    "reason": "validation complete"
                }),
            })),
            _ => Ok(Action::Message("Thinking...".to_string())),
        }
    }
}
