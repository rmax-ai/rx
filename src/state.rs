use async_trait::async_trait;
use anyhow::Result;
use crate::event::Event;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn load(&self, goal_id: &str) -> Result<Vec<Event>>;
    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()>;
}

pub struct InMemoryStateStore {
    events: Arc<Mutex<Vec<Event>>>,
    log_dir: PathBuf,
}

impl InMemoryStateStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            log_dir: PathBuf::from("logs"),
        }
    }
}

#[async_trait]
impl StateStore for InMemoryStateStore {
    async fn load(&self, _goal_id: &str) -> Result<Vec<Event>> {
        let events = self.events.lock().await;
        Ok(events.clone())
    }

    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()> {
        let mut events = self.events.lock().await;
        events.push(event.clone());

        // Write to file
        let log_path = self.log_dir.join(format!("{}.jsonl", goal_id));
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await?;
            
        let line = serde_json::to_string(&event)?;
        file.write_all(format!("{}\n", line).as_bytes()).await?;
        
        Ok(())
    }
}
