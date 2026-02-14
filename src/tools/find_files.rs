use crate::tool::Tool;
use crate::tools::fs_common::{display_path, is_hidden_name, metadata_modified_unix_ms};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;
use tokio::fs;

const DEFAULT_LIMIT: usize = 256;

#[derive(Debug, Serialize, Deserialize)]
struct FindFilesArgs {
    root: String,
    #[serde(default)]
    max_depth: Option<usize>,
    #[serde(default)]
    include_hidden: Option<bool>,
    #[serde(default)]
    extensions: Option<Vec<String>>,
    #[serde(default)]
    name_contains: Option<String>,
    #[serde(default)]
    path_contains: Option<String>,
    #[serde(default)]
    exclude_dirs: Option<Vec<String>>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    cursor: Option<String>,
}

#[derive(Debug)]
struct FileCandidate {
    relative_path: String,
    name: String,
    size: u64,
    modified_unix_ms: Option<u64>,
}

pub struct FindFilesTool;

#[async_trait]
impl Tool for FindFilesTool {
    fn name(&self) -> &'static str {
        "find_files"
    }

    fn description(&self) -> &'static str {
        "Recursively search for files with explicit filters, deterministic ordering, and truncation metadata."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "root": { "type": "string" },
                "max_depth": { "type": "integer", "minimum": 0 },
                "include_hidden": { "type": "boolean" },
                "extensions": { "type": "array", "items": { "type": "string" } },
                "name_contains": { "type": "string" },
                "path_contains": { "type": "string" },
                "exclude_dirs": { "type": "array", "items": { "type": "string" } },
                "limit": { "type": "integer", "minimum": 1 },
                "cursor": { "type": "string" }
            },
            "required": ["root"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: FindFilesArgs = serde_json::from_value(input)?;
        let root_path = Path::new(&args.root);
        let meta = fs::metadata(root_path)
            .await
            .map_err(|err| anyhow!("find_files failed to stat root {}: {}", args.root, err))?;
        if !meta.is_dir() {
            return Err(anyhow!("find_files root is not a directory: {}", args.root));
        }

        let extensions = args
            .extensions
            .as_ref()
            .map(|values| normalize_extensions(values));
        let name_contains = args
            .name_contains
            .as_ref()
            .map(|value| value.to_lowercase());
        let path_contains = args
            .path_contains
            .as_ref()
            .map(|value| value.to_lowercase());
        let exclude_dirs = args
            .exclude_dirs
            .as_ref()
            .map(|values| normalize_paths(values))
            .unwrap_or_default();
        let include_hidden = args.include_hidden.unwrap_or(false);
        let limit = args.limit.unwrap_or(DEFAULT_LIMIT).max(1);

        let mut candidates = Vec::new();
        collect_files(
            root_path,
            "",
            0,
            args.max_depth,
            include_hidden,
            extensions.as_ref(),
            name_contains.as_deref(),
            path_contains.as_deref(),
            &exclude_dirs,
            &mut candidates,
        )
        .await?;

        candidates.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let cursor_value = args
            .cursor
            .as_deref()
            .map(|value| normalize_rel_path(value));
        let mut filtered = Vec::new();
        let mut seen_cursor = cursor_value.is_none();
        let mut truncated = false;

        for candidate in candidates {
            if !seen_cursor {
                if let Some(cursor) = &cursor_value {
                    if candidate.relative_path <= *cursor {
                        continue;
                    }
                }
                seen_cursor = true;
            }

            filtered.push(candidate);
            if filtered.len() >= limit {
                truncated = true;
                break;
            }
        }

        let next_cursor = if truncated {
            filtered
                .last()
                .map(|candidate| candidate.relative_path.clone())
        } else {
            None
        };

        let entries: Vec<Value> = filtered
            .into_iter()
            .map(|candidate| {
                let absolute_path = root_path.join(&candidate.relative_path);
                serde_json::json!({
                    "path": display_path(&absolute_path),
                    "relative_path": candidate.relative_path,
                    "name": candidate.name,
                    "kind": "file",
                    "size": candidate.size,
                    "modified_unix_ms": candidate.modified_unix_ms
                })
            })
            .collect();

        Ok(serde_json::json!({
            "operation": "find_files",
            "root": args.root,
            "query": {
                "max_depth": args.max_depth,
                "include_hidden": include_hidden,
                "extensions": args.extensions,
                "name_contains": args.name_contains,
                "path_contains": args.path_contains,
                "exclude_dirs": args.exclude_dirs,
                "limit": limit,
                "cursor": args.cursor
            },
            "count": entries.len(),
            "truncated": truncated,
            "next_cursor": next_cursor,
            "entries": entries
        }))
    }
}

async fn collect_files(
    current: &Path,
    relative_prefix: &str,
    depth: usize,
    max_depth: Option<usize>,
    include_hidden: bool,
    extensions: Option<&HashSet<String>>,
    name_contains: Option<&str>,
    path_contains: Option<&str>,
    exclude_dirs: &[String],
    candidates: &mut Vec<FileCandidate>,
) -> Result<()> {
    if let Some(max) = max_depth {
        if depth > max {
            return Ok(());
        }
    }

    let mut entries = fs::read_dir(current).await?;
    let mut rows = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        rows.push((name, entry));
    }

    rows.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (name, entry) in rows {
        if !include_hidden && is_hidden_name(&name) {
            continue;
        }

        let metadata = entry.metadata().await?;
        let file_type = metadata.file_type();
        let relative_path = if relative_prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", relative_prefix, name)
        };
        let normalized_rel = normalize_rel_path(&relative_path);

        if file_type.is_dir() {
            if is_excluded_dir(&normalized_rel, &name, exclude_dirs) {
                continue;
            }
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
            }
            collect_files(
                &entry.path(),
                &normalized_rel,
                depth + 1,
                max_depth,
                include_hidden,
                extensions,
                name_contains,
                path_contains,
                exclude_dirs,
                candidates,
            )
            .await?;
        } else if file_type.is_file() {
            if !matches_extension(&entry.path(), extensions) {
                continue;
            }
            if let Some(name_filter) = name_contains {
                if !name.to_lowercase().contains(name_filter) {
                    continue;
                }
            }
            if let Some(path_filter) = path_contains {
                if !normalized_rel.to_lowercase().contains(path_filter) {
                    continue;
                }
            }

            candidates.push(FileCandidate {
                relative_path: normalized_rel,
                name: name.clone(),
                size: metadata.len(),
                modified_unix_ms: metadata_modified_unix_ms(&metadata),
            });
        }
    }

    Ok(())
}

fn matches_extension(path: &Path, extensions: Option<&HashSet<String>>) -> bool {
    if let Some(filters) = extensions {
        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .map(|value| value.trim_start_matches('.').to_lowercase());
        match extension {
            Some(ext) => filters.contains(&ext),
            None => false,
        }
    } else {
        true
    }
}

fn normalize_extensions(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .filter_map(|value| {
            let normalized = value.trim().trim_start_matches('.').to_lowercase();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
        .collect()
}

fn normalize_paths(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| normalize_rel_path(value))
        .collect()
}

fn normalize_rel_path(value: &str) -> String {
    value.replace('\\', "/").trim_matches('/').to_string()
}

fn is_excluded_dir(relative_path: &str, name: &str, exclusions: &[String]) -> bool {
    exclusions
        .iter()
        .any(|candidate| candidate == relative_path || candidate == name)
}
