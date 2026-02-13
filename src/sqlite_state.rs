use rusqlite::{params, Connection, Result};
use crate::state::StateStore;
use std::path::PathBuf;
use chrono::{Utc, DateTime};
use serde_json::Value as JsonValue;
use rusqlite::types::{FromSql, FromSqlResult, FromSqlError, ToSql, ToSqlOutput, ValueRef};

pub struct SqliteStateStore {
    conn: Connection,
}

struct SqliteJsonValue(JsonValue);

impl ToSql for SqliteJsonValue {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        Ok(ToSqlOutput::Owned(self.0.to_string().into_bytes()))
    }
}

impl FromSql for SqliteJsonValue {
    fn column_result(value: rusqlite::types::ValueRef) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => {
                let json_str = std::str::from_utf8(text).map_err(|_| FromSqlError::InvalidType)?;
                serde_json::from_str(json_str).map(SqliteJsonValue).map_err(|_| FromSqlError::InvalidType)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

struct SqliteDateTime(DateTime<Utc>);

impl ToSql for SqliteDateTime {
    fn to_sql(&self) -> Result<ToSqlOutput, rusqlite::Error> {
        Ok(ToSqlOutput::Owned(self.0.to_rfc3339().into_bytes()))
    }
}

impl FromSql for SqliteDateTime {
    fn column_result(value: rusqlite::types::ValueRef) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => {
                let date_str = std::str::from_utf8(text).map_err(|_| FromSqlError::InvalidType)?;
                DateTime::parse_from_rfc3339(date_str).map(|dt| SqliteDateTime(dt.with_timezone(&Utc))).map_err(|_| FromSqlError::InvalidType)
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
        Ok(SqliteStateStore { conn })
    }

    pub fn append_event(&self, goal_id: &str, r#type: &str, payload: &JsonValue, timestamp: &DateTime<Utc>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (goal_id, type, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![goal_id, r#type, &SqliteJsonValue(payload.clone()), &SqliteDateTime(*timestamp)],
        )?;
        Ok(())
    }

    pub fn load(&self, goal_id: &str) -> Result<Vec<(String, JsonValue, DateTime<Utc>)>> {
        let mut stmt = self.conn.prepare("SELECT type, payload, timestamp FROM events WHERE goal_id = ?1 ORDER BY id")?;
        let events = stmt.query_map(params![goal_id], |row| {
            Ok((row.get(0)?, row.get::<_, SqliteJsonValue>(1)?.0, row.get::<_, SqliteDateTime>(2)?.0))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    pub fn list_goals(&self) -> Result<Vec<(String, String)>> {
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
}