use async_trait::async_trait;
use anyhow::Result;
use crate::event::Event;
use rusqlite::{params, Connection};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use chrono::Utc;

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

        Ok(())
    }
}

pub struct SqliteStateStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStateStore {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY,
                goal_id TEXT NOT NULL,
                type TEXT NOT NULL,
                payload TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            params![],
        )?;
        Ok(SqliteStateStore { conn: Arc::new(Mutex::new(conn)) })
    }
}

#[async_trait]
impl StateStore for SqliteStateStore {
    async fn load(&self, goal_id: &str) -> Result<Vec<Event>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT type, payload, timestamp FROM events WHERE goal_id = ?1 ORDER BY timestamp")?;
        let events_iter = stmt.query_map(params![goal_id], |row| {
            Ok(Event {
                event_type: row.get(0)?,
                payload: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })?;
        let mut events = Vec::new();
        for event in events_iter {
            events.push(event?);
        }
        Ok(events)
    }

    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()> {
        let conn = self.conn.lock().await;
        let timestamp = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO events (goal_id, type, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![goal_id, event.event_type, event.payload, timestamp],
        )?;
        Ok(())
    }
}
