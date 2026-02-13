use crate::tool::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tokio::fs;

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
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write content to a file. Overwrites if exists."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: WriteFileArgs = serde_json::from_value(input)?;
        if let Some(parent) = Path::new(&args.path).parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&args.path, &args.content).await?;
        Ok(serde_json::json!({ "success": true, "bytes": args.content.len() }))
    }
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
