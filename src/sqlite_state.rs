use crate::event::Event;
use crate::state::StateStore;
use anyhow::Result;
use async_trait::async_trait;
use r2d2;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::PathBuf;

pub struct SqliteStateStore {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl SqliteStateStore {
    pub fn new(path: PathBuf) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path);
        let pool = r2d2::Pool::new(manager)?;
        let conn = pool.get()?;
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

        Ok(SqliteStateStore { pool })
    }
}

#[async_trait]
impl StateStore for SqliteStateStore {
    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO events (goal_id, type, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![
                goal_id,
                event.r#type,
                serde_json::to_string(&event.payload)?,
                event.timestamp
            ],
        )?;
        Ok(())
    }

    async fn load(&self, goal_id: &str) -> Result<Vec<Event>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT id, timestamp, type, payload FROM events WHERE goal_id = ?1 ORDER BY timestamp, id")?;
        let event_iter = stmt.query_map(params![goal_id], |row| {
            Ok(Event {
                id: row.get::<_, i64>(0)?.to_string(),
                timestamp: row.get(1)?,
                r#type: row.get(2)?,
                payload: {
                    let json_str: String = row.get(3)?;
                    serde_json::from_str(&json_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                },
            })
        })?;

        Ok(event_iter.collect::<Result<Vec<_>, _>>()?)
    }

    async fn list_goals(&self) -> Result<Vec<(String, String)>> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT DISTINCT goal_id, MAX(timestamp) FROM events GROUP BY goal_id")?;
        let goals = stmt
            .query_map(params![], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(goals)
    }
}
