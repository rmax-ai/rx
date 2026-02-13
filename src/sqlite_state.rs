use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use chrono::NaiveDateTime;
use async_trait::async_trait;
use super::Event;

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
                payload TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            params![],
        )?;
        Ok(SqliteStateStore { conn })
    }

    pub async fn list_goals(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT goal_id, MIN(timestamp) FROM events GROUP BY goal_id ORDER BY MIN(timestamp) DESC")?;
        let goal_iter = stmt.query_map(params![], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        let mut goals = Vec::new();
        for goal in goal_iter {
            goals.push(goal?);
        }
        Ok(goals)
    }

    pub fn append_event(&self, goal_id: &str, event_type: &str, payload: &str, timestamp: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (goal_id, type, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![goal_id, event_type, payload, timestamp],
        )?;
        Ok(())
    }

    pub fn load(&self, goal_id: &str) -> Result<Vec<(String, String, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT type, payload, timestamp FROM events WHERE goal_id = ?1 ORDER BY id")?;
        let events = stmt.query_map(params![goal_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?.collect::<Result<Vec<_>>>()?;
        Ok(events)
    }
}

#[async_trait]
impl StateStore for SqliteStateStore {
    async fn load(&self, goal_id: &str) -> Result<Vec<Event>> {
        self.load(goal_id).map(|events|{
            events.into_iter().map(|(event_type, payload, timestamp)|{
                Event {
                    event_type,
                    payload,
                    timestamp
                }
            }).collect()
        })
    }

    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()> {
        let timestamp = event.timestamp.clone();
        self.append_event(goal_id, &event.event_type, &event.payload, &timestamp)
    }
}
