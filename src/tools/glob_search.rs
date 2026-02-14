use crate::tool::Tool;
use crate::tools::fs_common::{
    display_path, is_hidden_name, metadata_modified_unix_ms, normalize_rel_path,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use globset::{GlobBuilder, GlobMatcher};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use tokio::fs;

const DEFAULT_MAX_RESULTS: usize = 256;

#[derive(Debug, Serialize, Deserialize)]
struct GlobSearchArgs {
    pattern: String,
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    include_hidden: Option<bool>,
    #[serde(default)]
    max_results: Option<usize>,
    #[serde(default)]
    cursor: Option<String>,
}

#[derive(Debug)]
struct MatchEntry {
    relative_path: String,
    name: String,
    kind: String,
    size: u64,
    modified_unix_ms: Option<u64>,
    absolute_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
enum KindFilter {
    Any,
    File,
    Dir,
    Symlink,
}

impl KindFilter {
    fn from_option(value: Option<&str>) -> Result<Self> {
        match value.map(|value| value.to_lowercase()).as_deref() {
            Some("file") => Ok(KindFilter::File),
            Some("dir") => Ok(KindFilter::Dir),
            Some("symlink") => Ok(KindFilter::Symlink),
            Some("any") | None => Ok(KindFilter::Any),
            Some(other) => Err(anyhow!(
                "glob_search kind filter must be one of: file, dir, symlink, any (got {})",
                other
            )),
        }
    }

    fn matches(&self, metadata: &Metadata) -> bool {
        let file_type = metadata.file_type();
        match self {
            KindFilter::Any => true,
            KindFilter::File => file_type.is_file(),
            KindFilter::Dir => file_type.is_dir(),
            KindFilter::Symlink => file_type.is_symlink(),
        }
    }
}

pub struct GlobSearchTool;

#[async_trait]
impl Tool for GlobSearchTool {
    fn name(&self) -> &'static str {
        "glob_search"
    }

    fn description(&self) -> &'static str {
        "Pattern-based discovery with deterministic ordering and cursor-aware truncation."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "root": { "type": "string" },
                "kind": { "type": "string", "enum": ["file", "dir", "symlink", "any"] },
                "include_hidden": { "type": "boolean" },
                "max_results": { "type": "integer", "minimum": 1 },
                "cursor": { "type": "string" }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: GlobSearchArgs = serde_json::from_value(input)?;
        let root_value = args.root.unwrap_or_else(|| ".".to_string());
        let root_path = Path::new(&root_value);
        let canonical_root = fs::canonicalize(root_path).await.map_err(|err| {
            anyhow!(
                "glob_search failed to canonicalize root {}: {}",
                root_value,
                err
            )
        })?;
        let root_meta = fs::metadata(&canonical_root).await?;
        if !root_meta.is_dir() {
            return Err(anyhow!(
                "glob_search root is not a directory: {}",
                root_value
            ));
        }

        let matcher = compile_pattern(&args.pattern)?;
        let kind_filter = KindFilter::from_option(args.kind.as_deref())?;
        let include_hidden = args.include_hidden.unwrap_or(false);
        let max_results = args.max_results.unwrap_or(DEFAULT_MAX_RESULTS).max(1);
        let cursor = args
            .cursor
            .as_deref()
            .map(|value| normalize_rel_path(value));
        let mut matches = Vec::new();

        collect_matches(
            &canonical_root,
            "",
            include_hidden,
            &matcher,
            kind_filter,
            &mut matches,
        )
        .await?;

        matches.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let mut filtered = Vec::new();
        let mut seen_cursor = cursor.is_none();
        let mut truncated = false;

        for entry in matches {
            if !seen_cursor {
                if let Some(cursor_value) = &cursor {
                    if entry.relative_path <= *cursor_value {
                        continue;
                    }
                }
                seen_cursor = true;
            }

            filtered.push(entry);
            if filtered.len() >= max_results {
                truncated = true;
                break;
            }
        }

        let next_cursor = if truncated {
            filtered.last().map(|entry| entry.relative_path.clone())
        } else {
            None
        };

        let entries_json: Vec<Value> = filtered
            .into_iter()
            .map(|entry| {
                serde_json::json!({
                    "path": display_path(&entry.absolute_path),
                    "relative_path": entry.relative_path,
                    "name": entry.name,
                    "kind": entry.kind,
                    "size": entry.size,
                    "modified_unix_ms": entry.modified_unix_ms
                })
            })
            .collect();

        Ok(serde_json::json!({
            "operation": "glob_search",
            "root": root_value,
            "query": {
                "pattern": args.pattern,
                "kind": args.kind,
                "include_hidden": include_hidden,
                "max_results": max_results,
                "cursor": args.cursor
            },
            "count": entries_json.len(),
            "truncated": truncated,
            "next_cursor": next_cursor,
            "entries": entries_json
        }))
    }
}

fn compile_pattern(pattern: &str) -> Result<GlobMatcher> {
    let glob = GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map_err(|err| anyhow!("glob_search invalid pattern {}: {}", pattern, err))?;
    Ok(glob.compile_matcher())
}

async fn collect_matches(
    current: &Path,
    relative_prefix: &str,
    include_hidden: bool,
    matcher: &GlobMatcher,
    kind_filter: KindFilter,
    matches: &mut Vec<MatchEntry>,
) -> Result<()> {
    let mut dir = fs::read_dir(current).await?;
    let mut rows = Vec::new();

    while let Some(entry) = dir.next_entry().await? {
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
        let matches_kind = kind_filter.matches(&metadata);
        let matches_pattern = matcher.is_match(&normalized_rel);

        if matches_kind && matches_pattern {
            matches.push(MatchEntry {
                relative_path: normalized_rel.clone(),
                name: name.clone(),
                kind: entry_kind_label(&file_type),
                size: metadata.len(),
                modified_unix_ms: metadata_modified_unix_ms(&metadata),
                absolute_path: entry.path(),
            });
        }

        if file_type.is_dir() {
            collect_matches(
                &entry.path(),
                &normalized_rel,
                include_hidden,
                matcher,
                kind_filter,
                matches,
            )
            .await?;
        }
    }

    Ok(())
}

fn entry_kind_label(file_type: &std::fs::FileType) -> String {
    if file_type.is_symlink() {
        "symlink".to_string()
    } else if file_type.is_dir() {
        "dir".to_string()
    } else if file_type.is_file() {
        "file".to_string()
    } else {
        "other".to_string()
    }
}
