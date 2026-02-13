use async_trait::async_trait;
use anyhow::Result;
use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use chrono::{Utc, DateTime};
use serde_json::Value;
use crate::event::Event;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn load(&self, goal_id: &str) -> Result<Vec<Event>>;
    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()>;
    async fn list_goals(&self) -> Result<Vec<(String, String)>>;
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

        Ok(())
    }
    
    async fn list_goals(&self) -> Result<Vec<(String, String)>> {
        // InMemory version of list_goals
        Ok(Vec::new())
    }
}

use crate::sqlite_state::{SqliteStateStore};

#[async_trait]
impl StateStore for SqliteStateStore {
    async fn load(&self, goal_id: &str) -> Result<Vec<Event>> {
        let events = self.load(goal_id)?;
        Ok(events.into_iter().map(|(r#type, payload, timestamp)|{
            Event {
                id: String::new(),
                r#type,
                payload,
                timestamp
            }
        }).collect())
    }

    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()> {
        self.append_event(goal_id, &event.r#type, &event.payload, &event.timestamp)
    }
    
    async fn list_goals(&self) -> Result<Vec<(String, String)>> {
        self.list_goals()
    }
}
