use crate::tool::Tool;
use crate::tools::fs_common::{display_path, kind_from_metadata, metadata_modified_unix_ms, EntryKind, is_hidden_name};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
struct ListDirEntriesArgs {
    path: String,
    include_hidden: Option<bool>,
}

#[derive(Debug)]
struct EntrySummary {
    name: String,
    path: String,
    kind: EntryKind,
    size: u64,
    modified_unix_ms: Option<u64>,
}

pub struct ListDirEntriesTool;

#[async_trait]
impl Tool for ListDirEntriesTool {
    fn name(&self) -> &'static str {
        "list_dir_entries"
    }

    fn description(&self) -> &'static str {
        "List directory entries with metadata and deterministic ordering."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "include_hidden": {
                    "type": "boolean",
                    "description": "Include entries whose names start with a dot."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: ListDirEntriesArgs = serde_json::from_value(input)?;
        let include_hidden = args.include_hidden.unwrap_or(false);
        let root_path = Path::new(&args.path);

        let mut read_dir = fs::read_dir(root_path).await?;
        let mut entries = Vec::new();

        while let Some(entry) = read_dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();

            if !include_hidden && is_hidden_name(&name) {
                continue;
            }

            entries.push(EntrySummary {
                name: name.clone(),
                path: display_path(&entry.path()),
                kind: kind_from_metadata(&metadata),
                size: metadata.len(),
                modified_unix_ms: metadata_modified_unix_ms(&metadata),
            });
        }

        entries.sort_by(|a, b| match a.kind.cmp(&b.kind) {
            Ordering::Equal => a.name.cmp(&b.name),
            other => other,
        });

        let entries_json: Vec<Value> = entries
            .into_iter()
            .map(|entry| {
                serde_json::json!({
                    "name": entry.name,
                    "path": entry.path,
                    "kind": entry.kind.as_str(),
                    "size": entry.size,
                    "modified_unix_ms": entry.modified_unix_ms
                })
            })
            .collect();

        Ok(serde_json::json!({
            "operation": "list_dir_entries",
            "root": args.path,
            "query": { "include_hidden": include_hidden },
            "count": entries_json.len(),
            "truncated": false,
            "entries": entries_json,
        }))
    }
}
