use crate::tool::Tool;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::{json, Map, Number, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::{metadata, read, read_dir, read_to_string, rename, remove_file, File, OpenOptions};
use tokio::io::AsyncWriteExt;

static TEMP_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
        let metadata = metadata(path).await.context("failed to stat file")?;
        let size_bytes = metadata.len();
        let mtime_unix_ms = metadata
            .modified()
            .ok()
            .and_then(system_time_to_unix_ms);
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

        let path_buf = PathBuf::from(path);

        if let Some(pre_val) = input.get("precondition") {
            let precondition = Precondition::try_from(pre_val).context("invalid precondition")?;
            if let Some(conflict) = precondition.evaluate(&path_buf).await.context("failed to evaluate precondition")? {
                return Ok(conflict);
            }
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
            map.insert("mtime_unix_ms".to_string(), Value::Number(Number::from(mtime)));
        }
        if let Some(size) = self.size_bytes {
            map.insert("size_bytes".to_string(), Value::Number(Number::from(size)));
        }
        map
    }
}

struct Precondition {
    expected_hash: Option<String>,
    expected_mtime_unix_ms: Option<i64>,
    expected_size_bytes: Option<u64>,
    require_all: bool,
}

impl Precondition {
    fn try_from(value: &Value) -> Result<Self> {
        let require_all = value
            .get("require_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let expected_hash = value
            .get("expected_hash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let expected_mtime_unix_ms = value.get("expected_mtime_unix_ms").and_then(|v| v.as_i64());
        let expected_size_bytes = value.get("expected_size_bytes").and_then(|v| v.as_u64());

        if require_all
            && (expected_hash.is_none()
                || expected_mtime_unix_ms.is_none()
                || expected_size_bytes.is_none())
        {
            return Err(anyhow!(
                "require_all precondition requires hash, mtime, and size"
            ));
        }

        Ok(Self {
            expected_hash,
            expected_mtime_unix_ms,
            expected_size_bytes,
            require_all,
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
                expected_map.insert("mtime_unix_ms".to_string(), Value::Number(Number::from(mtime)));
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
