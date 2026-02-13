use async_trait::async_trait;
use anyhow::Result;
use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use chrono::{Utc, DateTime};
use serde_json::Value;
use crate::event::Event;
use rusqlite::types::{FromSql, FromSqlResult, Type, ValueRef, ToSql, ToSqlOutput};
use rusqlite::Error as RusqliteError;

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

impl ToSql for Value {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, RusqliteError> {
        Ok(ToSqlOutput::Owned(self.to_string().into()))
    }
}

impl FromSql for Value {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => Ok(serde_json::from_str(std::str::from_utf8(text).unwrap()).unwrap()),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for DateTime<Utc> {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, RusqliteError> {
        Ok(ToSqlOutput::Owned(self.to_rfc3339().into()))
    }
}

impl FromSql for DateTime<Utc> {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => Ok(DateTime::parse_from_rfc3339(std::str::from_utf8(text).unwrap()).unwrap().with_timezone(&Utc)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[async_trait]
impl StateStore for SqliteStateStore {
    async fn load(&self, goal_id: &str) -> Result<Vec<Event>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT type, payload, timestamp FROM events WHERE goal_id = ?1 ORDER BY timestamp")?;
        let events_iter = stmt.query_map(params![goal_id], |row| {
            Ok(Event {
                id: String::new(), // Assuming ID is managed differently or could be skipped
                r#type: row.get(0)?,
                payload: row.get(1)?,
                timestamp: row.get(2)?
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
        let timestamp = event.timestamp.to_rfc3339();
        conn.execute(
            "INSERT INTO events (goal_id, type, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![goal_id, event.r#type, event.payload, timestamp],
        )?;
        Ok(())
    }
    
    async fn list_goals(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT DISTINCT goal_id, MIN(timestamp) FROM events GROUP BY goal_id ORDER BY MIN(timestamp) DESC")?;
        let goal_iter = stmt.query_map(params![], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        let mut goals = Vec::new();
        for goal in goal_iter {
            goals.push(goal?);
        }
        Ok(goals)
    }
}
