use crate::tool::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct DoneTool;

#[async_trait]
impl Tool for DoneTool {
    fn name(&self) -> &'static str {
        "done"
    }

    fn description(&self) -> &'static str {
        "Signal completion of the goal"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "reason": { "type": "string" },
                "details": { "type": ["object", "string", "null"] }
            }
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let reason = input
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("done");
        let details = input.get("details").cloned().unwrap_or(json!(null));
        Ok(json!({
            "status": "done",
            "reason": reason,
            "details": details,
        }))
    }
}
