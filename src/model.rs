use crate::event::Event;
use crate::tool::ToolRegistry;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    if let Some(text) = response_body.get("output_text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            return text.to_string();
        }
    }

    let mut content_chunks: Vec<String> = Vec::new();
    if let Some(output_items) = response_body.get("output").and_then(|v| v.as_array()) {
        for item in output_items {
            if item.get("type").and_then(|v| v.as_str()) != Some("message") {
                continue;
            }

            if let Some(content_items) = item.get("content").and_then(|v| v.as_array()) {
                for content in content_items {
                    if let Some(text) = content.get("text").and_then(|v| v.as_str()) {
                        if !text.is_empty() {
                            content_chunks.push(text.to_string());
                        }
                    }
                }
            }
        }
    }

    content_chunks.join("\n")
}

fn truncate_for_error(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    input.chars().take(max_chars).collect::<String>() + "..."
}

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

#[async_trait]
pub trait CommitMessageGenerator: Send + Sync {
    async fn commit_message(&self, diff: &str) -> Result<String>;
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
                    "name": t.name(),
                    "description": t.description(),
                    "parameters": t.parameters()
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
        let mut input = vec![json!({ "role": "developer", "content": self.system_prompt })];

        for event in history {
            match event.r#type.as_str() {
                "goal" => {
                    // Initial user goal
                    if let Some(content) = event.payload.get("goal").and_then(|v| v.as_str()) {
                        input.push(json!({ "role": "user", "content": content }));
                    }
                }
                "action" => {
                    // Assistant action
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
                    // Tool result
                    // Payload should have tool_call_id and output
                    let tool_call_id = event
                        .payload
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let output = event.payload.get("output").cloned().unwrap_or(Value::Null);
                    input.push(json!({
                        "role": "user",
                        "content": format!("tool_output tool_call_id={} output={}", tool_call_id, output)
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
            .context("Failed to send request to OpenAI")?;

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
            .context("Failed to read OpenAI response body")?;

        let response_body: Value = serde_json::from_str(&response_text).map_err(|err| {
            anyhow!(
                "Failed to parse OpenAI response JSON: status={} endpoint={} model={} request_id={} error={} body={}",
                status,
                endpoint,
                self.model_name,
                request_id,
                err,
                truncate_for_error(&response_text, 500)
            )
        })?;

        if let Some(output_items) = response_body.get("output").and_then(|v| v.as_array()) {
            for item in output_items {
                let item_type = item
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if item_type == "function_call" || item_type == "tool_call" {
                    let id = item
                        .get("call_id")
                        .or_else(|| item.get("id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();

                    let name = item
                        .get("name")
                        .or_else(|| item.get("function").and_then(|f| f.get("name")))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();

                    let args_val = match item.get("arguments") {
                        Some(Value::String(s)) => {
                            serde_json::from_str::<Value>(s).unwrap_or(json!({}))
                        }
                        Some(v @ Value::Object(_)) => v.clone(),
                        _ => json!({}),
                    };

                    return Ok(Action::ToolCall(ToolCall {
                        id,
                        name,
                        arguments: args_val,
                    }));
                }
            }
        }

        let content = parse_output_text(&response_body);
        Ok(Action::Message(content))
    }
}

pub struct OpenAICommitMessageModel {
    client: Client,
    api_key: String,
    model_name: String,
    system_prompt: String,
}

impl OpenAICommitMessageModel {
    pub fn new(api_key: String, model_name: String, system_prompt: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model_name,
            system_prompt,
        }
    }
}

#[async_trait]
impl CommitMessageGenerator for OpenAICommitMessageModel {
    async fn commit_message(&self, diff: &str) -> Result<String> {
        let endpoint = "https://api.openai.com/v1/responses";
        let input = vec![
            json!({ "role": "developer", "content": self.system_prompt }),
            json!({ "role": "user", "content": diff }),
        ];

        let request_body = json!({
            "model": self.model_name,
            "input": input
        });

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

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
            .context("Failed to read OpenAI response body")?;

        let response_body: Value = serde_json::from_str(&response_text).map_err(|err| {
            anyhow!(
                "Failed to parse OpenAI response JSON: status={} endpoint={} model={} request_id={} error={} body={}",
                status,
                endpoint,
                self.model_name,
                request_id,
                err,
                truncate_for_error(&response_text, 500)
            )
        })?;

        let content = parse_output_text(&response_body);
        let message = content
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .to_string();

        if message.is_empty() {
            Ok("rx: update".to_string())
        } else {
            Ok(message)
        }
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

pub struct MockCommitMessageModel;

#[async_trait]
impl CommitMessageGenerator for MockCommitMessageModel {
    async fn commit_message(&self, _diff: &str) -> Result<String> {
        Ok("rx: update".to_string())
    }
}
