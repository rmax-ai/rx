use crate::tool::Tool;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
struct ReadFileRangeArgs {
    path: String,
    start_line: usize,
    end_line: usize,
}

pub struct ReadFileRangeTool;

#[async_trait]
impl Tool for ReadFileRangeTool {
    fn name(&self) -> &'static str {
        "read_file_range"
    }

    fn description(&self) -> &'static str {
        "Read a deterministic range of lines from a file with explicit metadata."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the target file." },
                "start_line": { "type": "integer", "minimum": 1, "description": "Inclusive 1-based starting line." },
                "end_line": { "type": "integer", "minimum": 1, "description": "Inclusive 1-based ending line (>= start_line)." }
            },
            "required": ["path", "start_line", "end_line"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: ReadFileRangeArgs = serde_json::from_value(input)?;

        if args.start_line == 0 {
            return Err(anyhow!("start_line must be >= 1"));
        }
        if args.end_line < args.start_line {
            return Err(anyhow!("end_line must be >= start_line"));
        }

        let file = File::open(&args.path).await?;
        let mut reader = BufReader::new(file).lines();

        let mut current_line = 0usize;
        let mut collected = Vec::new();
        let mut truncated = false;
        let mut total_lines: Option<usize> = None;

        loop {
            match reader.next_line().await? {
                Some(line) => {
                    current_line += 1;
                    if current_line >= args.start_line && current_line <= args.end_line {
                        collected.push(line);
                    }

                    if current_line == args.end_line {
                        match reader.next_line().await? {
                            Some(_) => {
                                truncated = true;
                            }
                            None => {
                                total_lines = Some(current_line);
                            }
                        }
                        break;
                    }
                }
                None => {
                    total_lines = Some(current_line);
                    break;
                }
            }
        }

        let content = collected.join("\n");
        let line_count = collected.len();
        Ok(serde_json::json!({
            "operation": "read_file_range",
            "path": args.path,
            "start_line": args.start_line,
            "end_line": args.end_line,
            "requested_line_count": args.end_line - args.start_line + 1,
            "line_count": line_count,
            "total_lines": total_lines,
            "truncated": truncated,
            "lines": collected,
            "content": content,
        }))
    }
}
