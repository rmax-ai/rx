use crate::tool::Tool;
use crate::tools::fs_common::{kind_from_metadata, metadata_modified_unix_ms, EntryKind};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::ErrorKind;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
struct StatFileArgs {
    path: String,
}

pub struct StatFileTool;

#[async_trait]
impl Tool for StatFileTool {
    fn name(&self) -> &'static str {
        "stat_file"
    }

    fn description(&self) -> &'static str {
        "Retrieve file metadata without reading the entire file contents."
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
        let args: StatFileArgs = serde_json::from_value(input)?;
        let normalized_path = Path::new(&args.path);
        let metadata_result = fs::metadata(normalized_path).await;

        match metadata_result {
            Ok(metadata) => {
                let kind = kind_from_metadata(&metadata);
                Ok(serde_json::json!({
                    "operation": "stat_file",
                    "path": args.path,
                    "exists": true,
                    "kind": kind.as_str(),
                    "size_bytes": metadata.len(),
                    "modified_unix_ms": metadata_modified_unix_ms(&metadata),
                    "truncated": false,
                }))
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    Ok(serde_json::json!({
                        "operation": "stat_file",
                        "path": args.path,
                        "exists": false,
                        "kind": serde_json::Value::Null,
                        "size_bytes": serde_json::Value::Null,
                        "modified_unix_ms": serde_json::Value::Null,
                        "truncated": false,
                    }))
                } else {
                    Err(err.into())
                }
            }
        }
    }
}
