use crate::event::Event;
use crate::model::Action;
use crate::state::StateStore;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::to_string;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;

#[async_trait]
pub trait EventHook: Send + Sync {
    async fn on_event(&self, event: &Event) -> Result<()>;
}

pub struct HookedStateStore {
    inner: Arc<dyn StateStore>,
    hooks: Vec<Arc<dyn EventHook>>,
}

impl HookedStateStore {
    pub fn new(inner: Arc<dyn StateStore>, hooks: Vec<Arc<dyn EventHook>>) -> Self {
        Self { inner, hooks }
    }
}

#[async_trait]
impl StateStore for HookedStateStore {
    async fn load(&self) -> Result<Vec<Event>> {
        self.inner.load().await
    }

    async fn append_event(&self, event: Event) -> Result<()> {
        self.inner.append_event(event.clone()).await?;

        for hook in &self.hooks {
            if let Err(error) = hook.on_event(&event).await {
                eprintln!("Warning: event hook failed: {}", error);
            }
        }

        Ok(())
    }
}

pub struct DebugJsonlHook {
    writer: Mutex<File>,
}

impl DebugJsonlHook {
    pub async fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent)
                .await
                .with_context(|| format!("creating debug log directory {}", parent.display()))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .with_context(|| format!("opening debug log {}", path.display()))?;

        Ok(Self {
            writer: Mutex::new(file),
        })
    }
}

#[async_trait]
impl EventHook for DebugJsonlHook {
    async fn on_event(&self, event: &Event) -> Result<()> {
        let serialized = to_string(event).context("serializing debug event")?;
        let mut writer = self.writer.lock().await;
        writer.write_all(serialized.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }
}

pub struct ToolVerboseHook;

#[async_trait]
impl EventHook for ToolVerboseHook {
    async fn on_event(&self, event: &Event) -> Result<()> {
        match event.r#type.as_str() {
            "action" => {
                if let Ok(action) = serde_json::from_value::<Action>(event.payload.clone()) {
                    match action {
                        Action::Message(message) => {
                            println!("tool-verbose action message: {}", message);
                        }
                        Action::ToolCall(tool_call) => {
                            println!(
                                "tool-verbose tool input {} [{}]: {}",
                                tool_call.name, tool_call.id, tool_call.arguments
                            );
                        }
                    }
                }
            }
            "tool_output" => {
                if let Some(name) = event.payload.get("name").and_then(|value| value.as_str()) {
                    let output = event
                        .payload
                        .get("output")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    println!("tool-verbose tool output {}: {}", name, output);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[async_trait]
pub trait CommitMessageGenerator: Send + Sync {
    async fn commit_message(&self, diff: &str) -> Result<String>;
}

pub struct HeuristicCommitMessageGenerator;

#[async_trait]
impl CommitMessageGenerator for HeuristicCommitMessageGenerator {
    async fn commit_message(&self, diff: &str) -> Result<String> {
        let first_path = diff
            .lines()
            .find_map(|line| line.strip_prefix("+++ b/"))
            .filter(|path| !path.is_empty() && *path != "/dev/null");

        Ok(match first_path {
            Some(path) => format!("rx: update {}", path),
            None => "rx: update".to_string(),
        })
    }
}

pub struct AutoCommitHook {
    generator: Arc<dyn CommitMessageGenerator>,
}

impl AutoCommitHook {
    pub fn new(generator: Arc<dyn CommitMessageGenerator>) -> Self {
        Self { generator }
    }
}

#[async_trait]
impl EventHook for AutoCommitHook {
    async fn on_event(&self, event: &Event) -> Result<()> {
        if event.r#type != "tool_output" {
            return Ok(());
        }

        let tool_name = event
            .payload
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default();

        if tool_name == "done" {
            return Ok(());
        }

        let add_output = Command::new("git").args(["add", "."]).output().await?;
        if !add_output.status.success() {
            return Ok(());
        }

        let diff_check = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .output()
            .await?;

        let code = diff_check.status.code().unwrap_or(2);
        if code == 0 {
            return Ok(());
        }
        if code != 1 {
            return Ok(());
        }

        let diff_output = Command::new("git")
            .args(["diff", "--cached"])
            .output()
            .await?;
        if !diff_output.status.success() {
            return Ok(());
        }

        let diff_text = String::from_utf8_lossy(&diff_output.stdout).trim().to_string();
        if diff_text.is_empty() {
            return Ok(());
        }

        let message = self
            .generator
            .commit_message(&diff_text)
            .await
            .unwrap_or_else(|_| "rx: update".to_string());

        let _ = Command::new("git")
            .args(["commit", "-m", message.trim()])
            .output()
            .await?;

        Ok(())
    }
}
