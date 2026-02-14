use crate::event::Event;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::to_string;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn load(&self) -> Result<Vec<Event>>;
    async fn append_event(&self, event: Event) -> Result<()>;
}

pub struct InMemoryStateStore {
    events: Arc<Mutex<Vec<Event>>>,
    writer: Arc<Mutex<File>>,
    log_path: PathBuf,
}

impl InMemoryStateStore {
    pub async fn new(goal_id: &str) -> Result<Self> {
        let logs_dir = Path::new("logs");
        create_dir_all(logs_dir).await?;
        let log_path = logs_dir.join(format!("{}.jsonl", goal_id));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await
            .context("opening state log file")?;

        Ok(Self {
            events: Arc::new(Mutex::new(Vec::new())),
            writer: Arc::new(Mutex::new(file)),
            log_path,
        })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

#[async_trait]
impl StateStore for InMemoryStateStore {
    async fn load(&self) -> Result<Vec<Event>> {
        let events = self.events.lock().await;
        Ok(events.clone())
    }

    async fn append_event(&self, event: Event) -> Result<()> {
        {
            let mut events = self.events.lock().await;
            events.push(event.clone());
        }

        let serialized = to_string(&event).context("failed to serialize event for log")?;
        let mut writer = self.writer.lock().await;
        writer.write_all(serialized.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }
}
