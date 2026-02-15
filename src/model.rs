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

#[derive(Debug, Deserialize)]
struct OpenAIErrorEnvelope {
    error: OpenAIErrorBody,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorBody {
    message: String,
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    param: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

fn parse_output_text(response_body: &Value) -> String {
    if let Some(text) = response_body.get("output_text").and_then(|value| value.as_str()) {
        if !text.is_empty() {
            return text.to_string();
        }
    }

    let mut chunks: Vec<String> = Vec::new();
    if let Some(output_items) = response_body.get("output").and_then(|value| value.as_array()) {
        for item in output_items {
            if item.get("type").and_then(|value| value.as_str()) != Some("message") {
                continue;
            }

            if let Some(content_items) = item.get("content").and_then(|value| value.as_array()) {
                for content in content_items {
                    if let Some(text) = content.get("text").and_then(|value| value.as_str()) {
                        if !text.is_empty() {
                            chunks.push(text.to_string());
                        }
                    }
                }
            }
        }
    }

    chunks.join("\n")
}

fn truncate_for_error(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    input.chars().take(max_chars).collect::<String>() + "..."
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
            .map(|tool| {
                json!({
                    "type": "function",
                    "name": tool.name(),
                    "description": tool.description(),
                    "parameters": tool.parameters()
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

    fn events_to_input(&self, history: &[Event]) -> Vec<Value> {
        let mut input = vec![json!({
            "role": "developer",
            "content": self.system_prompt
        })];

        for event in history {
            match event.r#type.as_str() {
                "goal" => {
                    if let Some(goal) = event.payload.get("goal").and_then(|value| value.as_str()) {
                        input.push(json!({ "role": "user", "content": goal }));
                    }
                }
                "action" => {
                    if let Ok(action) = serde_json::from_value::<Action>(event.payload.clone()) {
                        match action {
                            Action::Message(content) => {
                                input.push(json!({ "role": "assistant", "content": content }));
                            }
                            Action::ToolCall(tool_call) => {
                                input.push(json!({
                                    "role": "assistant",
                                    "content": format!(
                                        "tool_call id={} name={} arguments={}",
                                        tool_call.id,
                                        tool_call.name,
                                        tool_call.arguments
                                    )
                                }));
                            }
                        }
                    }
                }
                "tool_output" => {
                    let tool_call_id = event
                        .payload
                        .get("tool_call_id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown");
                    let output = event
                        .payload
                        .get("output")
                        .cloned()
                        .unwrap_or(Value::Null);

                    input.push(json!({
                        "role": "user",
                        "content": format!(
                            "tool_output tool_call_id={} output={}",
                            tool_call_id,
                            output
                        )
                    }));
                }
                _ => {}
            }
        }

        input
    }
}

#[async_trait]
impl Model for OpenAIModel {
    async fn next_action(&self, history: &[Event]) -> Result<Action> {
        let endpoint = "https://api.openai.com/v1/responses";
        let input = self.events_to_input(history);

        let request_body = json!({
            "model": self.model_name,
            "input": input,
            "tools": self.tools,
            "tool_choice": "auto"
        });

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .context("failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status();
            let request_id = response
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("unknown")
                .to_string();
            let error_text = response.text().await.unwrap_or_default();

            if let Ok(error_envelope) = serde_json::from_str::<OpenAIErrorEnvelope>(&error_text) {
                let error = error_envelope.error;
                return Err(anyhow!(
                    "OpenAI API error: status={} endpoint={} model={} request_id={} type={} param={} code={} message={}",
                    status,
                    endpoint,
                    self.model_name,
                    request_id,
                    error.r#type.unwrap_or_else(|| "unknown".to_string()),
                    error.param.unwrap_or_else(|| "unknown".to_string()),
                    error.code.unwrap_or_else(|| "unknown".to_string()),
                    error.message
                ));
            }

            return Err(anyhow!(
                "OpenAI API error: status={} endpoint={} model={} request_id={} body={}",
                status,
                endpoint,
                self.model_name,
                request_id,
                truncate_for_error(&error_text, 500)
            ));
        }

        let status = response.status();
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        let response_text = response
            .text()
            .await
            .context("failed to read OpenAI response body")?;

        let response_body: Value = serde_json::from_str(&response_text).map_err(|error| {
            anyhow!(
                "Failed to parse OpenAI response JSON: status={} endpoint={} model={} request_id={} error={} body={}",
                status,
                endpoint,
                self.model_name,
                request_id,
                error,
                truncate_for_error(&response_text, 500)
            )
        })?;

        if let Some(output_items) = response_body.get("output").and_then(|value| value.as_array()) {
            for item in output_items {
                let item_type = item
                    .get("type")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if item_type == "function_call" || item_type == "tool_call" {
                    let id = item
                        .get("call_id")
                        .or_else(|| item.get("id"))
                        .and_then(|value| value.as_str())
                        .unwrap_or("call-unknown")
                        .to_string();
                    let name = item
                        .get("name")
                        .or_else(|| item.get("function").and_then(|value| value.get("name")))
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string();

                    let arguments = match item.get("arguments") {
                        Some(Value::String(json_text)) => {
                            serde_json::from_str::<Value>(json_text).unwrap_or(json!({}))
                        }
                        Some(value @ Value::Object(_)) => value.clone(),
                        _ => json!({}),
                    };

                    return Ok(Action::ToolCall(ToolCall {
                        id,
                        name,
                        arguments,
                    }));
                }
            }
        }

        Ok(Action::Message(parse_output_text(&response_body)))
    }
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
