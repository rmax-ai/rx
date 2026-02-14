use crate::tool::Tool;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
struct ReadFileHeadArgs {
    path: String,
    max_lines: usize,
}

pub struct ReadFileHeadTool;

#[async_trait]
impl Tool for ReadFileHeadTool {
    fn name(&self) -> &'static str {
        "read_file_head"
    }

    fn description(&self) -> &'static str {
        "Read the first N lines of a file with deterministic metadata."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "max_lines": { "type": "integer", "minimum": 1 }
            },
            "required": ["path", "max_lines"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: ReadFileHeadArgs = serde_json::from_value(input)?;
        if args.max_lines == 0 {
            return Err(anyhow!("max_lines must be >= 1"));
        }

        let file = File::open(&args.path).await?;
        let mut reader = BufReader::new(file).lines();
        let mut collected = Vec::new();
        let mut line_number = 0usize;
        let mut truncated = false;

        while let Some(line) = reader.next_line().await? {
            line_number += 1;
            if line_number <= args.max_lines {
                collected.push(line);
            } else {
                truncated = true;
            }
        }

        Ok(serde_json::json!({
            "operation": "read_file_head",
            "path": args.path,
            "max_lines": args.max_lines,
            "line_count": collected.len(),
            "total_lines": line_number,
            "truncated": truncated,
            "lines": collected,
            "content": collected.join("\n"),
        }))
    }
}
