use chrono::{DateTime, Utc};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef};
use rusqlite::{params, Connection, Result};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
