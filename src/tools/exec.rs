use crate::tool::Tool;
use crate::tools::exec_common::{execute_command, DEFAULT_MAX_STDERR_BYTES, DEFAULT_MAX_STDOUT_BYTES, ExecCommandRequest};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct ExecArgs {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<String>,
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
        "Execute a command with default bounded capture semantics. Use exec_status/exec_capture/exec_with_input for more targeted behaviors."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "args": { "type": "array", "items": { "type": "string" } },
                "cwd": { "type": "string" },
                "timeout_seconds": { "type": "integer" }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let ExecArgs {
            command,
            args,
            cwd,
            timeout_seconds,
        } = serde_json::from_value(input)?;

        let args_clone = args.clone();
        let result = execute_command(ExecCommandRequest {
            command: command.clone(),
            args,
            cwd: cwd.clone(),
            timeout_seconds,
            capture_stdout: true,
            capture_stderr: true,
            max_stdout_bytes: DEFAULT_MAX_STDOUT_BYTES,
            max_stderr_bytes: DEFAULT_MAX_STDERR_BYTES,
            stdin: None,
        })
        .await?;

        Ok(serde_json::json!({
            "operation": "exec",
            "command": command,
            "args": args_clone,
            "cwd": cwd,
            "exit_code": result.exit_code,
            "success": result.success,
            "timed_out": result.timed_out,
            "duration_ms": result.duration_ms,
            "stdout": result.stdout.unwrap_or_default(),
            "stderr": result.stderr.unwrap_or_default(),
            "stdout_truncated": result.stdout_truncated,
            "stderr_truncated": result.stderr_truncated
        }))
    }
}
