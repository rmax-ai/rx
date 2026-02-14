use crate::tool::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Debug, Serialize, Deserialize)]
struct ExecArgs {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
}

pub struct ExecTool;

#[async_trait]
impl Tool for ExecTool {
    fn name(&self) -> &'static str {
        "exec"
    }

    fn description(&self) -> &'static str {
        "Execute a system command. Does not run in a shell. The 'args' array must NOT include the command name itself."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The name of the executable to run (e.g., 'ls', 'rg', 'cargo')." },
                "args": { "type": "array", "items": { "type": "string" }, "description": "List of arguments to pass to the command. Do NOT include the command name as the first argument." },
                "timeout_seconds": { "type": "integer", "description": "Timeout in seconds (default 30)" }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: ExecArgs = serde_json::from_value(input)?;

        let mut cmd = Command::new(&args.command);
        cmd.args(&args.args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        let duration = Duration::from_secs(args.timeout_seconds.unwrap_or(30));

        let child = cmd.spawn()?;

        match timeout(duration, child.wait_with_output()).await {
            Ok(result) => {
                let output = result?;
                Ok(serde_json::json!({
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                    "exit_code": output.status.code(),
                    "success": output.status.success(),
                }))
            }
            Err(_) => Ok(serde_json::json!({
                "error": "timeout",
                "success": false
            })),
        }
    }
}
