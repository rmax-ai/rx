use crate::tool::Tool;
use crate::tools::exec_common::{execute_command, ExecCommandRequest, STATUS_STDERR_BYTES};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct ExecStatusArgs {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<String>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
}

pub struct ExecStatusTool;

#[async_trait]
impl Tool for ExecStatusTool {
    fn name(&self) -> &'static str {
        "exec_status"
    }

    fn description(&self) -> &'static str {
        "Run a command and return only structured status metadata."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The executable to run." },
                "args": { "type": "array", "items": { "type": "string" } },
                "cwd": { "type": "string", "description": "Optional working directory." },
                "timeout_seconds": { "type": "integer", "description": "Optional timeout override in seconds." }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let ExecStatusArgs {
            command,
            args,
            cwd,
            timeout_seconds,
        } = serde_json::from_value(input)?;

        let result = execute_command(ExecCommandRequest {
            command: command.clone(),
            args: args.clone(),
            cwd: cwd.clone(),
            timeout_seconds,
            capture_stdout: false,
            capture_stderr: true,
            max_stdout_bytes: 0,
            max_stderr_bytes: STATUS_STDERR_BYTES,
            stdin: None,
        })
        .await?;

        Ok(serde_json::json!({
            "operation": "exec_status",
            "command": command,
            "args": args,
            "cwd": cwd,
            "exit_code": result.exit_code,
            "success": result.success,
            "timed_out": result.timed_out,
            "duration_ms": result.duration_ms,
            "stderr": result.stderr.unwrap_or_default(),
            "stderr_truncated": result.stderr_truncated
        }))
    }
}
