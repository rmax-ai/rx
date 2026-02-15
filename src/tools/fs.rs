use crate::tool::Tool;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use diffy::{apply, Patch};
use serde_json::{json, Map, Number, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::{
    create_dir_all, metadata, read, read_dir, read_to_string, remove_file, rename, OpenOptions,
};
use tokio::io::AsyncWriteExt;

static TEMP_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct ListDirTool;
pub struct CreateFileTool;
pub struct AppendFileTool;
pub struct ReplaceInFileTool;
pub struct ApplyPatchTool;
pub struct ApplyUnifiedPatchTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read an entire UTF-8 text file and return content plus metadata (hash, mtime, size). Use this to inspect current file state before planning edits."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Read full text content of a file path.",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to read."
                }
            },
            "required": ["path"],
            "examples": [
                { "path": "README.md" },
                { "path": "src/main.rs" },
                { "path": ".rx/config.toml" }
            ]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'path' parameter is required"))?;
        let contents = read_to_string(path).await.context("failed to read file")?;
        let metadata = metadata(path).await.context("failed to stat file")?;
        let size_bytes = metadata.len();
        let mtime_unix_ms = metadata.modified().ok().and_then(system_time_to_unix_ms);
        let hash = compute_hash(contents.as_bytes());

        Ok(json!({
            "content": contents,
            "metadata": {
                "hash": hash,
                "mtime_unix_ms": mtime_unix_ms,
                "size_bytes": size_bytes
            }
        }))
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write UTF-8 text to a file with deterministic modes. `overwrite` replaces file atomically; `append` appends bytes to the end."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Write file content, optionally guarded by file-state preconditions.",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Target file path."
                },
                "content": {
                    "type": "string",
                    "description": "Content to write."
                },
                "mode": {
                    "type": "string",
                    "enum": ["overwrite", "append"],
                    "description": "Write mode. Defaults to `overwrite`."
                },
                "expected_hash": {
                    "type": "string",
                    "description": "Optional optimistic-concurrency guard. Write proceeds only if current hash matches."
                },
                "expected_mtime_unix_ms": {
                    "type": "integer",
                    "description": "Optional mtime precondition in Unix milliseconds."
                },
                "expected_size_bytes": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional size precondition in bytes."
                }
            },
            "required": ["path", "content"],
            "examples": [
                {
                    "path": "notes/todo.txt",
                    "content": "- finish evals\n",
                    "mode": "append"
                },
                {
                    "path": "src/lib.rs",
                    "content": "pub fn answer() -> u32 { 42 }\n",
                    "mode": "overwrite"
                },
                {
                    "path": "README.md",
                    "content": "# rx\n",
                    "expected_hash": "3f1f4f6dbe8d...."
                }
            ]
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

        let path_buf = PathBuf::from(path);

        if let Some(conflict) = apply_precondition(&input, &path_buf).await? {
            return Ok(conflict);
        }

        if mode == "append" {
            let mut options = OpenOptions::new();
            options.write(true).create(true).append(true);
            let mut file = options
                .open(&path_buf)
                .await
                .context("failed to open target file")?;
            file.write_all(content.as_bytes())
                .await
                .context("failed to write content")?;
            file.flush().await.context("failed to flush file")?;
        } else {
            write_atomically(&path_buf, content.as_bytes())
                .await
                .context("failed to perform atomic write")?;
        }

        Ok(json!({ "path": path, "mode": mode }))
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &'static str {
        "list_dir"
    }

    fn description(&self) -> &'static str {
        "List immediate directory entries and classify each as file, dir, or other. Use this for path discovery before reads/writes."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "List one directory level (non-recursive).",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to inspect. Defaults to current directory when omitted."
                }
            },
            "examples": [
                { "path": "." },
                { "path": "src/tools" },
                { "path": "plans" }
            ]
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

#[async_trait]
impl Tool for CreateFileTool {
    fn name(&self) -> &'static str {
        "create_file"
    }

    fn description(&self) -> &'static str {
        "Create a new file with atomic write semantics. Fails with `already_exists` if the target path already exists."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Create a brand-new file only.",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path for the new file."
                },
                "content": {
                    "type": "string",
                    "description": "Initial file content."
                },
                "expected_hash": {
                    "type": "string",
                    "description": "Optional precondition guard."
                },
                "expected_mtime_unix_ms": {
                    "type": "integer",
                    "description": "Optional precondition guard."
                },
                "expected_size_bytes": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional precondition guard."
                }
            },
            "required": ["path", "content"],
            "examples": [
                {
                    "path": "docs/notes.md",
                    "content": "# Notes\n"
                },
                {
                    "path": "tmp/output.txt",
                    "content": "generated at runtime\n"
                }
            ]
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
        let path_buf = PathBuf::from(path);

        if let Some(conflict) = apply_precondition(&input, &path_buf).await? {
            return Ok(conflict);
        }

        if metadata(&path_buf).await.is_ok() {
            return Ok(json!({
                "success": false,
                "error": "already_exists",
                "path": path
            }));
        }

        write_atomically(&path_buf, content.as_bytes())
            .await
            .context("failed to create file atomically")?;

        Ok(json!({ "path": path, "created": true }))
    }
}

#[async_trait]
impl Tool for AppendFileTool {
    fn name(&self) -> &'static str {
        "append_file"
    }

    fn description(&self) -> &'static str {
        "Append UTF-8 content to a file, creating it if missing. Useful for logs, incremental notes, and non-destructive updates."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Append content to end-of-file.",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Target file path."
                },
                "content": {
                    "type": "string",
                    "description": "Text to append."
                },
                "expected_hash": {
                    "type": "string",
                    "description": "Optional optimistic-concurrency guard."
                },
                "expected_mtime_unix_ms": {
                    "type": "integer",
                    "description": "Optional mtime precondition."
                },
                "expected_size_bytes": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional size precondition."
                }
            },
            "required": ["path", "content"],
            "examples": [
                {
                    "path": "CHANGELOG.md",
                    "content": "\n- Added deterministic tool examples"
                },
                {
                    "path": "logs/run.log",
                    "content": "iteration=4 status=ok\n"
                }
            ]
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
        let path_buf = PathBuf::from(path);

        if let Some(conflict) = apply_precondition(&input, &path_buf).await? {
            return Ok(conflict);
        }

        let mut options = OpenOptions::new();
        options.write(true).create(true).append(true);
        let mut file = options
            .open(&path_buf)
            .await
            .context("failed to open target file")?;
        file.write_all(content.as_bytes())
            .await
            .context("failed to append content")?;
        file.flush().await.context("failed to flush file")?;

        Ok(json!({
            "path": path,
            "appended_bytes": content.as_bytes().len()
        }))
    }
}

#[async_trait]
impl Tool for ReplaceInFileTool {
    fn name(&self) -> &'static str {
        "replace_in_file"
    }

    fn description(&self) -> &'static str {
        "Replace exact text matches in a file with match-count protection. Use `expected_matches` to prevent accidental broad edits."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Perform deterministic textual replacement.",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to modify."
                },
                "old_text": {
                    "type": "string",
                    "description": "Exact text to find."
                },
                "new_text": {
                    "type": "string",
                    "description": "Replacement text."
                },
                "expected_matches": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Required number of matches. Defaults to 1."
                },
                "expected_hash": {
                    "type": "string",
                    "description": "Optional optimistic-concurrency guard."
                },
                "expected_mtime_unix_ms": {
                    "type": "integer",
                    "description": "Optional mtime precondition."
                },
                "expected_size_bytes": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional size precondition."
                }
            },
            "required": ["path", "old_text", "new_text"],
            "examples": [
                {
                    "path": "src/main.rs",
                    "old_text": "max_iterations = 25",
                    "new_text": "max_iterations = 50",
                    "expected_matches": 1
                },
                {
                    "path": "README.md",
                    "old_text": "gpt-4o",
                    "new_text": "gpt-5",
                    "expected_matches": 2
                }
            ]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'path' parameter is required"))?;
        let old_text = input
            .get("old_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'old_text' parameter is required"))?;
        let new_text = input
            .get("new_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'new_text' parameter is required"))?;
        let expected_matches = input
            .get("expected_matches")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);
        let path_buf = PathBuf::from(path);

        if let Some(conflict) = apply_precondition(&input, &path_buf).await? {
            return Ok(conflict);
        }

        let contents = read_to_string(&path_buf)
            .await
            .context("failed to read target file")?;
        let found = contents.matches(old_text).count();

        if found != expected_matches {
            return Ok(json!({
                "success": false,
                "error": "unexpected_match_count",
                "path": path,
                "expected_matches": expected_matches,
                "actual_matches": found
            }));
        }

        let replaced = replace_n(&contents, old_text, new_text, expected_matches);

        write_atomically(&path_buf, replaced.as_bytes())
            .await
            .context("failed to write replaced content")?;

        Ok(json!({
            "path": path,
            "replaced_matches": expected_matches
        }))
    }
}

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &'static str {
        "apply_patch"
    }

    fn description(&self) -> &'static str {
        "Use the `apply_patch` shell command to edit files.\nYour patch language is a stripped-down, file-oriented diff format designed to be easy to parse and safe to apply."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Apply a file-oriented patch envelope:\n*** Begin Patch\n*** Add File: <path>\n*** Delete File: <path>\n*** Update File: <path>\n*** End Patch\nPaths must be relative.",
            "properties": {
                "patch": {
                    "type": "string",
                    "description": "Patch text in apply_patch format."
                }
            },
            "required": ["patch"],
            "examples": [
                {
                    "patch": "*** Begin Patch\n*** Add File: hello.txt\n+Hello world\n*** End Patch\n"
                },
                {
                    "patch": "*** Begin Patch\n*** Update File: src/app.py\n*** Move to: src/main.py\n@@ def greet():\n-print(\"Hi\")\n+print(\"Hello, world!\")\n*** End Patch\n"
                }
            ]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let patch_text = input
            .get("patch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'patch' parameter is required"))?;

        let patch_ops = parse_apply_patch(patch_text).context("failed to parse patch")?;
        let summary = apply_patch_ops(&patch_ops).await.context("failed to apply patch")?;

        Ok(json!({
            "patched": true,
            "added_files": summary.added_files,
            "updated_files": summary.updated_files,
            "deleted_files": summary.deleted_files,
            "moved_files": summary.moved_files
        }))
    }
}

#[async_trait]
impl Tool for ApplyUnifiedPatchTool {
    fn name(&self) -> &'static str {
        "apply_unified_patch"
    }

    fn description(&self) -> &'static str {
        "Apply a unified diff to a single target file. Use when edits are easier to express as contextual hunks than full rewrites."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Apply unified patch text to an existing file.",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Target file path."
                },
                "patch": {
                    "type": "string",
                    "description": "Unified diff patch text to apply."
                },
                "expected_hash": {
                    "type": "string",
                    "description": "Optional optimistic-concurrency guard."
                },
                "expected_mtime_unix_ms": {
                    "type": "integer",
                    "description": "Optional mtime precondition."
                },
                "expected_size_bytes": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Optional size precondition."
                }
            },
            "required": ["path", "patch"],
            "examples": [
                {
                    "path": "src/lib.rs",
                    "patch": "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@\n-pub fn old() {}\n+pub fn new() {}\n"
                },
                {
                    "path": "README.md",
                    "patch": "--- a/README.md\n+++ b/README.md\n@@\n-Old line\n+New line\n"
                }
            ]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'path' parameter is required"))?;
        let patch_text = input
            .get("patch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("'patch' parameter is required"))?;
        let path_buf = PathBuf::from(path);

        if let Some(conflict) = apply_precondition(&input, &path_buf).await? {
            return Ok(conflict);
        }

        let base_content = read_to_string(&path_buf)
            .await
            .context("failed to read target file")?;
        let patch = Patch::from_str(patch_text).context("failed to parse patch")?;
        let patched = apply(&base_content, &patch).context("failed to apply patch")?;

        write_atomically(&path_buf, patched.as_bytes())
            .await
            .context("failed to write patched content")?;

        Ok(json!({ "path": path, "patched": true }))
    }
}

#[derive(Default)]
struct FileMetadata {
    hash: Option<String>,
    mtime_unix_ms: Option<i64>,
    size_bytes: Option<u64>,
}

impl FileMetadata {
    fn to_map(&self) -> Map<String, Value> {
        let mut map = Map::new();
        if let Some(hash) = &self.hash {
            map.insert("hash".to_string(), Value::String(hash.clone()));
        }
        if let Some(mtime) = self.mtime_unix_ms {
            map.insert(
                "mtime_unix_ms".to_string(),
                Value::Number(Number::from(mtime)),
            );
        }
        if let Some(size) = self.size_bytes {
            map.insert("size_bytes".to_string(), Value::Number(Number::from(size)));
        }
        map
    }
}

struct TempFileGuard {
    path: PathBuf,
    disarmed: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            disarmed: false,
        }
    }

    fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.disarmed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

struct Precondition {
    expected_hash: Option<String>,
    expected_mtime_unix_ms: Option<i64>,
    expected_size_bytes: Option<u64>,
}

impl Precondition {
    fn try_from(value: &Value) -> Result<Self> {
        let expected_hash = value
            .get("expected_hash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let expected_mtime_unix_ms = value.get("expected_mtime_unix_ms").and_then(|v| v.as_i64());
        let expected_size_bytes = value.get("expected_size_bytes").and_then(|v| v.as_u64());

        Ok(Self {
            expected_hash,
            expected_mtime_unix_ms,
            expected_size_bytes,
        })
    }

    async fn evaluate(&self, path: &Path) -> Result<Option<Value>> {
        let actual = gather_file_metadata(path).await?;

        let mut mismatch = false;
        if let Some(expected_hash) = &self.expected_hash {
            if actual.hash.as_deref() != Some(expected_hash.as_str()) {
                mismatch = true;
            }
        }
        if let Some(expected_mtime) = self.expected_mtime_unix_ms {
            if actual.mtime_unix_ms != Some(expected_mtime) {
                mismatch = true;
            }
        }
        if let Some(expected_size) = self.expected_size_bytes {
            if actual.size_bytes != Some(expected_size) {
                mismatch = true;
            }
        }

        if mismatch {
            let mut expected_map = Map::new();
            if let Some(hash) = &self.expected_hash {
                expected_map.insert("hash".to_string(), Value::String(hash.clone()));
            }
            if let Some(mtime) = self.expected_mtime_unix_ms {
                expected_map.insert(
                    "mtime_unix_ms".to_string(),
                    Value::Number(Number::from(mtime)),
                );
            }
            if let Some(size) = self.expected_size_bytes {
                expected_map.insert("size_bytes".to_string(), Value::Number(Number::from(size)));
            }

            let conflict = json!({
                "success": false,
                "error": "precondition_failed",
                "path": path.to_string_lossy().to_string(),
                "expected": Value::Object(expected_map),
                "actual": Value::Object(actual.to_map()),
            });
            return Ok(Some(conflict));
        }

        Ok(None)
    }
}

async fn gather_file_metadata(path: &Path) -> Result<FileMetadata> {
    let metadata = match metadata(path).await {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(FileMetadata::default()),
        Err(err) => return Err(err.into()),
    };

    let size_bytes = metadata.len();
    let mtime_unix_ms = metadata.modified().ok().and_then(system_time_to_unix_ms);

    let hash = match read(path).await {
        Ok(bytes) => Some(compute_hash(&bytes)),
        Err(_) => None,
    };

    Ok(FileMetadata {
        hash,
        mtime_unix_ms,
        size_bytes: Some(size_bytes),
    })
}

async fn apply_precondition(input: &Value, path: &Path) -> Result<Option<Value>> {
    if let Some(pre_val) = input.get("precondition") {
        let precondition = Precondition::try_from(pre_val).context("invalid precondition")?;
        return precondition
            .evaluate(path)
            .await
            .context("failed to evaluate precondition");
    }

    Ok(None)
}

fn replace_n(source: &str, from: &str, to: &str, mut remaining: usize) -> String {
    if remaining == 0 {
        return source.to_string();
    }

    let mut output = String::with_capacity(source.len());
    let mut rest = source;

    while remaining > 0 {
        if let Some(idx) = rest.find(from) {
            output.push_str(&rest[..idx]);
            output.push_str(to);
            rest = &rest[idx + from.len()..];
            remaining -= 1;
        } else {
            break;
        }
    }

    output.push_str(rest);
    output
}

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut hash = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(hash, "{:02x}", byte).unwrap();
    }
    hash
}

fn system_time_to_unix_ms(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|dur| dur.as_millis() as i64)
}

async fn write_atomically(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target");
    let temp_name = format!(
        ".rx-write-{}-{}",
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst),
        file_name
    );
    let temp_path = parent.join(temp_name);

    let mut guard = TempFileGuard::new(temp_path.clone());

    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .await
        .context("failed to create temporary file")?;
    temp_file
        .write_all(data)
        .await
        .context("failed to write to temporary file")?;
    temp_file
        .sync_all()
        .await
        .context("failed to sync temporary file")?;

    rename(&temp_path, path)
        .await
        .context("failed to rename temporary file")?;

    guard.disarm();
    sync_parent_dir(parent).await;

    Ok(())
}

async fn sync_parent_dir(parent: &Path) {
    let _ = OpenOptions::new().read(true).open(parent).await;
}

#[derive(Debug)]
enum ApplyPatchOp {
    Add {
        path: String,
        lines: Vec<String>,
    },
    Delete {
        path: String,
    },
    Update {
        path: String,
        move_to: Option<String>,
        hunks: Vec<ApplyPatchHunk>,
    },
}

#[derive(Debug, Clone)]
struct ApplyPatchHunk {
    lines: Vec<ApplyPatchHunkLine>,
}

#[derive(Debug, Clone)]
enum ApplyPatchHunkLine {
    Context(String),
    Remove(String),
    Add(String),
}

#[derive(Default)]
struct ApplyPatchSummary {
    added_files: usize,
    updated_files: usize,
    deleted_files: usize,
    moved_files: usize,
}

fn parse_apply_patch(input: &str) -> Result<Vec<ApplyPatchOp>> {
    let lines: Vec<&str> = input
        .lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();

    if lines.is_empty() {
        return Err(anyhow!("patch is empty"));
    }

    let mut index = 0usize;
    expect_line(&lines, index, "*** Begin Patch")?;
    index += 1;

    let mut ops = Vec::new();
    while index < lines.len() {
        let line = lines[index];
        if line == "*** End Patch" {
            index += 1;
            if index != lines.len() {
                return Err(anyhow!("unexpected content after *** End Patch"));
            }
            return Ok(ops);
        }

        if let Some(path) = line.strip_prefix("*** Add File: ") {
            validate_relative_path(path)?;
            index += 1;
            let mut added = Vec::new();
            while index < lines.len() {
                let current = lines[index];
                if is_patch_header(current) || current == "*** End Patch" {
                    break;
                }
                let content = current
                    .strip_prefix('+')
                    .ok_or_else(|| anyhow!("invalid add-file line: expected '+' prefix"))?;
                added.push(content.to_string());
                index += 1;
            }

            ops.push(ApplyPatchOp::Add {
                path: path.to_string(),
                lines: added,
            });
            continue;
        }

        if let Some(path) = line.strip_prefix("*** Delete File: ") {
            validate_relative_path(path)?;
            ops.push(ApplyPatchOp::Delete {
                path: path.to_string(),
            });
            index += 1;
            continue;
        }

        if let Some(path) = line.strip_prefix("*** Update File: ") {
            validate_relative_path(path)?;
            index += 1;

            let mut move_to = None;
            if index < lines.len() {
                if let Some(target) = lines[index].strip_prefix("*** Move to: ") {
                    validate_relative_path(target)?;
                    move_to = Some(target.to_string());
                    index += 1;
                }
            }

            let mut hunks = Vec::new();
            while index < lines.len() {
                let current = lines[index];
                if is_patch_header(current) || current == "*** End Patch" {
                    break;
                }

                if !current.starts_with("@@") {
                    return Err(anyhow!("expected hunk header '@@', got: {}", current));
                }

                index += 1;
                let mut hunk_lines = Vec::new();
                while index < lines.len() {
                    let hline = lines[index];
                    if hline.starts_with("@@") || is_patch_header(hline) || hline == "*** End Patch"
                    {
                        break;
                    }
                    if hline == "*** End of File" {
                        index += 1;
                        break;
                    }

                    let mut chars = hline.chars();
                    let marker = chars
                        .next()
                        .ok_or_else(|| anyhow!("empty hunk line is invalid"))?;
                    let tail: String = chars.collect();
                    match marker {
                        ' ' => hunk_lines.push(ApplyPatchHunkLine::Context(tail)),
                        '-' => hunk_lines.push(ApplyPatchHunkLine::Remove(tail)),
                        '+' => hunk_lines.push(ApplyPatchHunkLine::Add(tail)),
                        _ => return Err(anyhow!("invalid hunk line prefix '{}'", marker)),
                    }
                    index += 1;
                }

                if hunk_lines.is_empty() {
                    return Err(anyhow!("empty hunk is invalid"));
                }
                hunks.push(ApplyPatchHunk { lines: hunk_lines });
            }

            if hunks.is_empty() {
                return Err(anyhow!("update operation for '{}' has no hunks", path));
            }

            ops.push(ApplyPatchOp::Update {
                path: path.to_string(),
                move_to,
                hunks,
            });
            continue;
        }

        return Err(anyhow!("unknown patch section header: {}", line));
    }

    Err(anyhow!("missing *** End Patch"))
}

fn expect_line(lines: &[&str], index: usize, expected: &str) -> Result<()> {
    let found = lines
        .get(index)
        .copied()
        .ok_or_else(|| anyhow!("patch ended early; expected {}", expected))?;
    if found != expected {
        return Err(anyhow!("expected '{}', got '{}'", expected, found));
    }
    Ok(())
}

fn is_patch_header(line: &str) -> bool {
    line.starts_with("*** Add File: ")
        || line.starts_with("*** Delete File: ")
        || line.starts_with("*** Update File: ")
}

fn validate_relative_path(path: &str) -> Result<()> {
    if path.trim().is_empty() {
        return Err(anyhow!("path cannot be empty"));
    }

    let parsed = Path::new(path);
    if parsed.is_absolute() {
        return Err(anyhow!("path must be relative: {}", path));
    }

    for component in parsed.components() {
        match component {
            Component::CurDir | Component::Normal(_) => {}
            Component::ParentDir => return Err(anyhow!("parent path '..' is not allowed: {}", path)),
            _ => return Err(anyhow!("invalid path component in {}", path)),
        }
    }
    Ok(())
}

async fn apply_patch_ops(ops: &[ApplyPatchOp]) -> Result<ApplyPatchSummary> {
    let mut summary = ApplyPatchSummary::default();

    for op in ops {
        match op {
            ApplyPatchOp::Add { path, lines } => {
                let target = PathBuf::from(path);
                if metadata(&target).await.is_ok() {
                    return Err(anyhow!("add file failed: '{}' already exists", path));
                }
                if let Some(parent) = target.parent() {
                    create_dir_all(parent)
                        .await
                        .with_context(|| format!("failed to create parent directories for {}", path))?;
                }
                write_atomically(&target, normalize_patch_lines(lines).as_bytes())
                    .await
                    .with_context(|| format!("failed to write {}", path))?;
                summary.added_files += 1;
            }
            ApplyPatchOp::Delete { path } => {
                let target = PathBuf::from(path);
                if metadata(&target).await.is_err() {
                    return Err(anyhow!("delete file failed: '{}' does not exist", path));
                }
                remove_file(&target)
                    .await
                    .with_context(|| format!("failed to delete {}", path))?;
                summary.deleted_files += 1;
            }
            ApplyPatchOp::Update {
                path,
                move_to,
                hunks,
            } => {
                let source_path = PathBuf::from(path);
                let original = read_to_string(&source_path)
                    .await
                    .with_context(|| format!("failed to read {}", path))?;
                let updated = apply_patch_hunks(&original, hunks)
                    .with_context(|| format!("failed to patch {}", path))?;

                let dest_path = move_to
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| source_path.clone());

                if let Some(parent) = dest_path.parent() {
                    create_dir_all(parent).await.with_context(|| {
                        format!(
                            "failed to create parent directories for {}",
                            dest_path.display()
                        )
                    })?;
                }

                write_atomically(&dest_path, updated.as_bytes())
                    .await
                    .with_context(|| format!("failed to write {}", dest_path.display()))?;

                if let Some(target) = move_to {
                    if target != path {
                        remove_file(&source_path).await.with_context(|| {
                            format!("failed to remove moved source file {}", path)
                        })?;
                        summary.moved_files += 1;
                    }
                }
                summary.updated_files += 1;
            }
        }
    }

    Ok(summary)
}

fn apply_patch_hunks(original: &str, hunks: &[ApplyPatchHunk]) -> Result<String> {
    let mut lines: Vec<String> = original
        .lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect();
    let mut cursor = 0usize;

    for hunk in hunks {
        let expected_old: Vec<&str> = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                ApplyPatchHunkLine::Context(text) | ApplyPatchHunkLine::Remove(text) => {
                    Some(text.as_str())
                }
                ApplyPatchHunkLine::Add(_) => None,
            })
            .collect();

        let replacement: Vec<String> = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                ApplyPatchHunkLine::Context(text) | ApplyPatchHunkLine::Add(text) => {
                    Some(text.clone())
                }
                ApplyPatchHunkLine::Remove(_) => None,
            })
            .collect();

        let match_pos = find_hunk_match(&lines, &expected_old, cursor)
            .or_else(|| find_hunk_match(&lines, &expected_old, 0))
            .ok_or_else(|| anyhow!("could not locate hunk context in target file"))?;

        let old_len = expected_old.len();
        lines.splice(match_pos..(match_pos + old_len), replacement.clone());
        cursor = match_pos + replacement.len();
    }

    let mut output = lines.join("\n");
    if original.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

fn find_hunk_match(lines: &[String], expected_old: &[&str], start: usize) -> Option<usize> {
    if expected_old.is_empty() {
        return Some(start.min(lines.len()));
    }
    if expected_old.len() > lines.len() || start > lines.len() {
        return None;
    }

    let end = lines.len() - expected_old.len();
    for idx in start..=end {
        let window = &lines[idx..idx + expected_old.len()];
        if window.iter().zip(expected_old.iter()).all(|(a, b)| a == b) {
            return Some(idx);
        }
    }
    None
}

fn normalize_patch_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
