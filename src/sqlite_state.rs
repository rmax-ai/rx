use async_trait::async_trait;
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use crate::event::Event;
use crate::state::{StateStore};

pub struct SqliteStateStore {
    conn: Connection,
}

impl SqliteStateStore {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY,
                goal_id TEXT NOT NULL,
                type TEXT NOT NULL,
                payload TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            params![],
        )?;
        
        Ok(SqliteStateStore { conn })
    }
}

#[async_trait]
impl StateStore for SqliteStateStore {

    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (goal_id, type, payload) VALUES (?1, ?2, ?3)",
            params![goal_id, event.event_type, serde_json::to_string(&event.payload)?],
        )?;
        Ok(())
    }

    async fn load(&self, goal_id: &str) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare("SELECT type, payload FROM events WHERE goal_id = ?1 ORDER BY timestamp, id")?;
        let event_iter = stmt.query_map(params![goal_id], |row| {
            Ok(Event {
                event_type: row.get(0)?,
                payload: serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(1)?)?,
            })
        })?;
        
        Ok(event_iter.collect::<Result<Vec<_>, _>>()?)
    }

    async fn list_goals(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT goal_id, MAX(timestamp) FROM events GROUP BY goal_id")?;
        let goals = stmt.query_map(params![], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(goals)
    }
}
