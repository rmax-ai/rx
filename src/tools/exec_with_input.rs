use crate::tool::Tool;
use crate::tools::exec_common::{
    execute_command, ExecCommandRequest, DEFAULT_MAX_STDERR_BYTES, DEFAULT_MAX_STDOUT_BYTES,
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
struct ExecWithInputArgs {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<String>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
    #[serde(default)]
    stdin: Option<String>,
    #[serde(default)]
    max_stdout_bytes: Option<usize>,
    #[serde(default)]
    max_stderr_bytes: Option<usize>,
}

impl ExecWithInputArgs {
    fn resolved_stdout_limit(&self) -> usize {
        self.max_stdout_bytes.unwrap_or(DEFAULT_MAX_STDOUT_BYTES)
    }

    fn resolved_stderr_limit(&self) -> usize {
        self.max_stderr_bytes.unwrap_or(DEFAULT_MAX_STDERR_BYTES)
    }
}

pub struct ExecWithInputTool;

#[async_trait]
impl Tool for ExecWithInputTool {
    fn name(&self) -> &'static str {
        "exec_with_input"
    }

    fn description(&self) -> &'static str {
        "Run a command with deterministic stdin payload and bounded capture."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "args": { "type": "array", "items": { "type": "string" } },
                "cwd": { "type": "string" },
                "timeout_seconds": { "type": "integer" },
                "stdin": { "type": "string", "description": "Optional stdin payload." },
                "max_stdout_bytes": { "type": "integer" },
                "max_stderr_bytes": { "type": "integer" }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: ExecWithInputArgs = serde_json::from_value(input)?;
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
            stdin: args.stdin.clone(),
        })
        .await?;

        Ok(serde_json::json!({
            "operation": "exec_with_input",
            "command": args.command,
            "args": args.args,
            "cwd": args.cwd,
            "exit_code": result.exit_code,
            "success": result.success,
            "timed_out": result.timed_out,
            "duration_ms": result.duration_ms,
            "stdin": args.stdin,
            "stdout": result.stdout.unwrap_or_default(),
            "stderr": result.stderr.unwrap_or_default(),
            "stdout_truncated": result.stdout_truncated,
            "stderr_truncated": result.stderr_truncated
        }))
    }
}
