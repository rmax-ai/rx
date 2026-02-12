use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;
use crate::tool::Tool;

#[derive(Debug, Serialize, Deserialize)]
struct DoneArgs {
    reason: String,
}

pub struct DoneTool;

#[async_trait]
impl Tool for DoneTool {
    fn name(&self) -> &'static str {
        "done"
    }

    fn description(&self) -> &'static str {
        "Signal that the goal is achieved or execution must stop."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "reason": { "type": "string" }
            },
            "required": ["reason"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: DoneArgs = serde_json::from_value(input)?;
        Ok(serde_json::json!({ "status": "done", "reason": args.reason }))
    }
}
