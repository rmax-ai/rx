use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

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

    pub fn append_event(&self, goal_id: &str, event_type: &str, payload: &str, timestamp: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (goal_id, type, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![goal_id, event_type, payload, timestamp],
        )?;
        Ok(())
    }

    pub fn load(&self, goal_id: &str) -> Result<Vec<(String, String, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT type, payload, timestamp FROM events WHERE goal_id = ?1 ORDER BY timestamp")?;
        let events = stmt.query_map(params![goal_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?.collect::<Result<Vec<_>>>()?;
        Ok(events)
    }
}
