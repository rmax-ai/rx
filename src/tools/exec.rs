use crate::tool::Tool;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

pub struct ExecTool;

#[async_trait]
impl Tool for ExecTool {
    fn name(&self) -> &'static str {
        "exec"
    }

    fn description(&self) -> &'static str {
        "Run a single executable (no shell expansion) and capture stdout, stderr, exit status, and exit code. Use this for deterministic command execution when you know the exact binary and arguments."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Execute a process directly. Prefer explicit args over shell strings for safety and replayability.",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Executable name or path. Example: `rg`, `cargo`, `git`."
                },
                "args": {
                    "type": "array",
                    "description": "Positional arguments passed exactly as provided.",
                    "items": { "type": "string" }
                },
                "cwd": {
                    "type": "string",
                    "description": "Optional working directory for the command."
                }
            },
            "required": ["command"],
            "examples": [
                {
                    "command": "rg",
                    "args": ["--files", "src"]
                },
                {
                    "command": "cargo",
                    "args": ["test"],
                    "cwd": "."
                },
                {
                    "command": "git",
                    "args": ["status", "--short"]
                }
            ]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'command' field is required"))?;

        let args = input
            .get("args")
            .and_then(|v| v.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        let mut cmd = Command::new(command);
        cmd.args(&args);

        if let Some(cwd) = input.get("cwd").and_then(|v| v.as_str()) {
            cmd.current_dir(cwd);
        }

        let output = cmd.output().await.context("failed to execute command")?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "status": output.status.to_string(),
            "code": output.status.code(),
        }))
    }
}
