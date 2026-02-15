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
        "Signal that work is complete and request loop termination. Include a concise reason and optional structured details summarizing final outcome, checks, or artifacts."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "description": "Use only when the task is complete or cannot progress further.",
            "properties": {
                "reason": {
                    "type": "string",
                    "description": "Short completion reason. Example: `implemented feature and verified tests`."
                },
                "details": {
                    "type": ["object", "string", "null"],
                    "description": "Optional structured summary of results."
                }
            },
            "examples": [
                {
                    "reason": "goal achieved",
                    "details": {
                        "files_updated": 3,
                        "tests": "cargo test passed"
                    }
                },
                {
                    "reason": "blocked by missing credentials",
                    "details": "Cannot continue without API key."
                },
                {
                    "reason": "done"
                }
            ]
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
