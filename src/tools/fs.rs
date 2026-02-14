use crate::tool::Tool;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::{metadata, read_dir, read_to_string, rename, remove_file, File, OpenOptions};
use tokio::io::AsyncWriteExt;

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct ListDirTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read a file from the workspace"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'path' parameter is required"))?;
        let contents = read_to_string(path).await.context("failed to read file")?;
        Ok(json!({ "content": contents }))
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write content to a file"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" },
                "mode": { "type": "string", "enum": ["overwrite", "append"] }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'path' parameter is required"))?;
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'content' parameter is required"))?;
        let mode = input
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("overwrite");

        let mut options = OpenOptions::new();
        options.write(true).create(true);
        if mode == "append" {
            options.append(true);
        } else {
            options.truncate(true);
        }

        let mut file = options
            .open(path)
            .await
            .context("failed to open target file")?;
        file.write_all(content.as_bytes())
            .await
            .context("failed to write content")?;
        file.flush().await.context("failed to flush file")?;

        Ok(json!({ "path": path, "mode": mode }))
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &'static str {
        "list_dir"
    }

    fn description(&self) -> &'static str {
        "List entries in a directory"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            }
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let mut entries = Vec::new();
        let mut dir = read_dir(path).await.context("failed to read directory")?;
        while let Some(entry) = dir
            .next_entry()
            .await
            .context("failed to read directory entry")?
        {
            let file_type = entry
                .file_type()
                .await
                .context("failed to inspect entry type")?;
            let kind = if file_type.is_dir() {
                "dir"
            } else if file_type.is_file() {
                "file"
            } else {
                "other"
            };
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(json!({
                "name": name,
                "kind": kind
            }));
        }

        Ok(json!({ "entries": entries }))
    }
}
