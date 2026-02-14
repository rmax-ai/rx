use crate::tool::Tool;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct WhichCommandArgs {
    command: String,
}

pub struct WhichCommandTool;

#[async_trait]
impl Tool for WhichCommandTool {
    fn name(&self) -> &'static str {
        "which_command"
    }

    fn description(&self) -> &'static str {
        "Resolve an executable path using the system PATH."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The executable name to resolve." }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: WhichCommandArgs = serde_json::from_value(input)?;
        let command = args.command;
        let paths = env::var_os("PATH");

        if let Some(paths) = paths {
            for dir in env::split_paths(&paths) {
                let candidate = dir.join(&command);
                if candidate.exists() {
                    return Ok(serde_json::json!({
                        "found": true,
                        "command": command,
                        "path": candidate.to_string_lossy(),
                    }));
                }
            }
        }

        Err(anyhow!("{} not found in PATH", command))
    }
}
