use crate::tool::Tool;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Serialize, Deserialize)]
struct ReadFileArgs {
    path: String,
}

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read contents of a file."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: ReadFileArgs = serde_json::from_value(input)?;
        let content = fs::read_to_string(&args.path).await?;
        Ok(serde_json::json!({ "content": content }))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WriteFileArgs {
    path: String,
    content: String,
    mode: Option<String>,
    force: Option<bool>,
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write content to a file with explicit mode. Supports create, overwrite, append."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" },
                "mode": {
                    "type": "string",
                    "enum": ["create", "overwrite", "append"],
                    "description": "Write strategy. create: fail if file exists, overwrite: replace full file, append: append to end."
                },
                "force": {
                    "type": "boolean",
                    "description": "Bypass overwrite safety checks when true."
                }
            },
            "required": ["path", "content", "mode"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: WriteFileArgs = serde_json::from_value(input)?;
        let mode = args.mode.unwrap_or_else(|| "overwrite".to_string());
        let force = args.force.unwrap_or(false);

        if let Some(parent) = Path::new(&args.path).parent() {
            fs::create_dir_all(parent).await?;
        }

        match mode.as_str() {
            "create" => {
                if fs::try_exists(&args.path).await? {
                    return Err(anyhow!(
                        "write_file(create) refused: file already exists at {}",
                        args.path
                    ));
                }
                fs::write(&args.path, &args.content).await?;
            }
            "overwrite" => {
                if fs::try_exists(&args.path).await? && !force {
                    let existing_size = fs::metadata(&args.path).await?.len() as usize;
                    let new_size = args.content.len();

                    if existing_size > 0 && new_size * 100 < existing_size * 60 {
                        return Err(anyhow!(
                            "write_file(overwrite) refused: new content is much smaller than existing file ({} -> {} bytes). Use force=true if intentional.",
                            existing_size,
                            new_size
                        ));
                    }

                    if looks_like_partial_content(&args.content) {
                        return Err(anyhow!(
                            "write_file(overwrite) refused: content appears to contain placeholder/partial markers. Provide complete content or use a patch-style edit."
                        ));
                    }
                }

                fs::write(&args.path, &args.content).await?;
            }
            "append" => {
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&args.path)
                    .await?;
                file.write_all(args.content.as_bytes()).await?;
                file.flush().await?;
            }
            _ => {
                return Err(anyhow!(
                    "Invalid mode '{}'. Expected one of: create, overwrite, append.",
                    mode
                ));
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "bytes": args.content.len(),
            "mode": mode,
            "forced": force
        }))
    }
}

fn looks_like_partial_content(content: &str) -> bool {
    let markers = [
        "...existing code...",
        "... omitted ...",
        "Omitted for brevity",
        "// Omitted",
        "/* Omitted",
        "TODO: add rest",
    ];

    let lowered = content.to_lowercase();
    markers
        .iter()
        .any(|marker| lowered.contains(&marker.to_lowercase()))
}

#[derive(Debug, Serialize, Deserialize)]
struct ListDirArgs {
    path: String,
}

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &'static str {
        "list_dir"
    }

    fn description(&self) -> &'static str {
        "List entries in a directory."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: ListDirArgs = serde_json::from_value(input)?;
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&args.path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let meta = entry.metadata().await?;
            let kind = if meta.is_dir() { "dir" } else { "file" };
            entries.push(serde_json::json!({
                "name": entry.file_name().to_string_lossy(),
                "kind": kind
            }));
        }

        Ok(serde_json::json!({ "entries": entries }))
    }
}
