use async_trait::async_trait;
use anyhow::Result;
use rusqlite::params;
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
        let goal_id = goal_id.to_string();
        let conn_arc = Arc::clone(&self.conn);
        let events = tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT type, payload, timestamp FROM events WHERE goal_id = ?1 ORDER BY id",
            )?;
            let events = stmt
                .query_map(params![goal_id], |row| {
                    Ok((
                        row.get(0)?,
                        row.get::<_, crate::sqlite_state::SqliteJsonValue>(1)?.0,
                        row.get::<_, crate::sqlite_state::SqliteDateTime>(2)?.0,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<(String, Value, DateTime<Utc>)>, rusqlite::Error>(events)
        }).await??;
        Ok(events.into_iter().map(|(r#type, payload, timestamp)|{
            Event {
                id: String::new(),
                r#type,
                payload,
                timestamp
            }
        }).collect())
    }

    async fn append_event(&self, _goal_id: &str, event: Event) -> Result<()> {
        let goal_id = _goal_id.to_string();
        let r#type = event.r#type.clone();
        let payload = event.payload.clone();
        let timestamp = event.timestamp;
        let conn_arc = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                "INSERT INTO events (goal_id, type, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
                params![goal_id, r#type, &crate::sqlite_state::SqliteJsonValue(payload), &crate::sqlite_state::SqliteDateTime(timestamp)],
            )?;
            Ok::<(), rusqlite::Error>(())
        }).await??;
        Ok(())
    }
    
    async fn list_goals(&self) -> Result<Vec<(String, String)>> {
        let conn_arc = Arc::clone(&self.conn);
        Ok(tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().unwrap();
            let mut stmt = conn.prepare("SELECT DISTINCT goal_id, MIN(timestamp) FROM events GROUP BY goal_id ORDER BY MIN(timestamp) DESC")?;
            let goal_iter = stmt.query_map(params![], |row| Ok((row.get(0)?, row.get(1)?)))?;
            let mut goals = Vec::new();
            for goal in goal_iter {
                goals.push(goal?);
            }
            Ok::<Vec<(String, String)>, rusqlite::Error>(goals)
        }).await??)
    }
}
