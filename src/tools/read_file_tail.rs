use crate::tool::Tool;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
struct ReadFileTailArgs {
    path: String,
    max_lines: usize,
}

pub struct ReadFileTailTool;

#[async_trait]
impl Tool for ReadFileTailTool {
    fn name(&self) -> &'static str {
        "read_file_tail"
    }

    fn description(&self) -> &'static str {
        "Read the last N lines of a file with deterministic metadata."
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
        let args: ReadFileTailArgs = serde_json::from_value(input)?;
        if args.max_lines == 0 {
            return Err(anyhow!("max_lines must be >= 1"));
        }

        let file = File::open(&args.path).await?;
        let mut reader = BufReader::new(file).lines();
        let mut window = Vec::new();
        let mut total_lines = 0usize;

        while let Some(line) = reader.next_line().await? {
            total_lines += 1;
            window.push(line);
            if window.len() > args.max_lines {
                window.remove(0);
            }
        }

        let truncated = total_lines > args.max_lines;
        let lines = window.clone();

        Ok(serde_json::json!({
            "operation": "read_file_tail",
            "path": args.path,
            "max_lines": args.max_lines,
            "line_count": lines.len(),
            "total_lines": total_lines,
            "truncated": truncated,
            "lines": lines,
            "content": window.join("\n"),
        }))
    }
}
