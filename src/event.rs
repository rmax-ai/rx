use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub r#type: String,
    pub payload: serde_json::Value,
}

impl Event {
    pub fn new(r#type: &str, payload: serde_json::Value) -> Self {
        Self {
            id: Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
                .to_string(),
            timestamp: Utc::now(),
            r#type: r#type.to_string(),
            payload,
        }
    }
}
