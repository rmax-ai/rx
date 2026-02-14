use crate::tool::Tool;
use crate::tools::exec_common::{execute_command, DEFAULT_MAX_STDERR_BYTES, DEFAULT_MAX_STDOUT_BYTES, ExecCommandRequest};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct ExecCaptureArgs {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<String>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
    #[serde(default)]
    max_stdout_bytes: Option<usize>,
    #[serde(default)]
    max_stderr_bytes: Option<usize>,
}

impl ExecCaptureArgs {
    fn resolved_stdout_limit(&self) -> usize {
        self.max_stdout_bytes.unwrap_or(DEFAULT_MAX_STDOUT_BYTES)
    }

    fn resolved_stderr_limit(&self) -> usize {
        self.max_stderr_bytes.unwrap_or(DEFAULT_MAX_STDERR_BYTES)
    }
}

pub struct ExecCaptureTool;

#[async_trait]
impl Tool for ExecCaptureTool {
    fn name(&self) -> &'static str {
        "exec_capture"
    }

    fn description(&self) -> &'static str {
        "Run a command and capture bounded stdout/stderr for reasoning."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "args": { "type": "array", "items": { "type": "string" } },
                "cwd": { "type": "string" },
                "timeout_seconds": { "type": "integer" },
                "max_stdout_bytes": { "type": "integer", "description": "Optional cap for stdout capture." },
                "max_stderr_bytes": { "type": "integer", "description": "Optional cap for stderr capture." }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: ExecCaptureArgs = serde_json::from_value(input)?;
        let stdout_limit = args.resolved_stdout_limit();
        let stderr_limit = args.resolved_stderr_limit();
        let result = execute_command(ExecCommandRequest {
            command: args.command.clone(),
            args: args.args.clone(),
            cwd: args.cwd.clone(),
            timeout_seconds: args.timeout_seconds,
            capture_stdout: true,
            capture_stderr: true,
            max_stdout_bytes: stdout_limit,
            max_stderr_bytes: stderr_limit,
            stdin: None,
        })
        .await?;

        Ok(serde_json::json!({
            "operation": "exec_capture",
            "command": args.command,
            "args": args.args,
            "cwd": args.cwd,
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
