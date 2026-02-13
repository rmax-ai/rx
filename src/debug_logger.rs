use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

pub struct DebugLogger {
    file: Mutex<tokio::fs::File>,
}

impl DebugLogger {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                create_dir_all(parent).await?;
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        Ok(Self {
            file: Mutex::new(file),
        })
    }

    pub async fn log(&self, entry: &Value) -> Result<()> {
        let mut file = self.file.lock().await;
        let mut line = serde_json::to_string(entry)?;
        if !line.ends_with('\n') {
            line.push('\n');
        }
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }
}
