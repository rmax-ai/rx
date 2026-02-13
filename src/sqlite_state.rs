use chrono::{DateTime, Utc};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef};
use rusqlite::{params, Connection, Result};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use crate::event::Event;
use crate::state::StateStore;

pub struct SqliteStateStore {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

pub(crate) struct SqliteJsonValue(pub JsonValue);

impl ToSql for SqliteJsonValue {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, rusqlite::Error> {
        Ok(ToSqlOutput::Owned(Value::Text(self.0.to_string())))
    }
}

impl FromSql for SqliteJsonValue {
    fn column_result(value: rusqlite::types::ValueRef) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => {
                let json_str = std::str::from_utf8(text).map_err(|_| FromSqlError::InvalidType)?;
                serde_json::from_str(json_str)
                    .map(SqliteJsonValue)
                    .map_err(|_| FromSqlError::InvalidType)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

pub(crate) struct SqliteDateTime(pub DateTime<Utc>);

impl ToSql for SqliteDateTime {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>, rusqlite::Error> {
        Ok(ToSqlOutput::Owned(Value::Text(self.0.to_rfc3339())))
    }
}

impl FromSql for SqliteDateTime {
    fn column_result(value: rusqlite::types::ValueRef) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => {
                let date_str = std::str::from_utf8(text).map_err(|_| FromSqlError::InvalidType)?;
                DateTime::parse_from_rfc3339(date_str)
                    .map(|dt| SqliteDateTime(dt.with_timezone(&Utc)))
                    .map_err(|_| FromSqlError::InvalidType)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
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
        Ok(SqliteStateStore {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl StateStore for SqliteStateStore {
    async fn load(&self, goal_id: &str) -> anyhow::Result<Vec<Event>> {
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
                        row.get::<_, SqliteJsonValue>(1)?.0,
                        row.get::<_, SqliteDateTime>(2)?.0,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<Vec<(String, JsonValue, DateTime<Utc>)>, rusqlite::Error>(events)
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

    async fn append_event(&self, _goal_id: &str, event: Event) -> anyhow::Result<()> {
        let goal_id = _goal_id.to_string();
        let r#type = event.r#type.clone();
        let payload = event.payload.clone();
        let timestamp = event.timestamp;
        let conn_arc = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                "INSERT INTO events (goal_id, type, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
                params![goal_id, r#type, &SqliteJsonValue(payload), &SqliteDateTime(timestamp)],
            )?;
            Ok::<(), rusqlite::Error>(())
        }).await??;
        Ok(())
    }
    
    async fn list_goals(&self) -> anyhow::Result<Vec<(String, String)>> {
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
