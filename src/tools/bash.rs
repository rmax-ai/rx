use crate::tool::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Debug, Serialize, Deserialize)]
struct BashArgs {
    script: String,
    #[serde(default)]
    timeout_seconds: Option<u64>,
}

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Execute a bash script. The script runs in /bin/bash -c. Useful for complex commands, pipes, and redirects."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "script": { "type": "string", "description": "The bash script to execute." },
                "timeout_seconds": { "type": "integer", "description": "Timeout in seconds (default 30)" }
            },
            "required": ["script"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: BashArgs = serde_json::from_value(input)?;

        let mut cmd = Command::new("/bin/bash");
        cmd.arg("-c");
        cmd.arg(&args.script);
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
