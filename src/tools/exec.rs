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
        "Execute a command and capture stdout/stderr"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "args": { "type": "array", "items": { "type": "string" } },
                "cwd": { "type": "string" }
            },
            "required": ["command"]
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

        let output = cmd
            .output()
            .await
            .context("failed to execute command")?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "status": output.status.to_string(),
            "code": output.status.code(),
        }))
    }
}
