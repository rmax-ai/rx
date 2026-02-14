use crate::tool::Tool;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
struct SearchInFileArgs {
    path: String,
    query: String,
    #[serde(default)]
    is_regex: Option<bool>,
    #[serde(default)]
    max_matches: Option<usize>,
    #[serde(default)]
    before_lines: Option<usize>,
    #[serde(default)]
    after_lines: Option<usize>,
    #[serde(default)]
    case_sensitive: Option<bool>,
}

pub struct SearchInFileTool;

#[async_trait]
impl Tool for SearchInFileTool {
    fn name(&self) -> &'static str {
        "search_in_file"
    }

    fn description(&self) -> &'static str {
        "Find literal or regex matches in a file with context metadata."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "query": { "type": "string" },
                "is_regex": { "type": "boolean" },
                "case_sensitive": { "type": "boolean" },
                "max_matches": { "type": "integer", "minimum": 1 },
                "before_lines": { "type": "integer", "minimum": 0 },
                "after_lines": { "type": "integer", "minimum": 0 }
            },
            "required": ["path", "query"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let args: SearchInFileArgs = serde_json::from_value(input)?;
        if args.query.trim().is_empty() {
            return Err(anyhow!("query must not be empty"));
        }

        let max_matches = args.max_matches.unwrap_or(10).max(1);
        let before_lines = args.before_lines.unwrap_or(0);
        let after_lines = args.after_lines.unwrap_or(0);
        let case_sensitive = args.case_sensitive.unwrap_or(true);
        let is_regex = args.is_regex.unwrap_or(false);

        let regex = if is_regex {
            let mut builder = RegexBuilder::new(&args.query);
            builder.case_insensitive(!case_sensitive);
            Some(builder.build()?)
        } else {
            None
        };

        let content = fs::read_to_string(&args.path).await?;
        let lines: Vec<String> = content.lines().map(|value| value.to_string()).collect();
        let total_lines = lines.len();
        let mut matches = Vec::new();
        let mut truncated = false;
        let run_query = if case_sensitive {
            args.query.clone()
        } else {
            args.query.to_lowercase()
        };

        for (index, line) in lines.iter().enumerate() {
            if matches.len() >= max_matches {
                truncated = true;
                break;
            }

            let mut match_text = None;
            if let Some(regexp) = &regex {
                if let Some(captures) = regexp.find(line) {
                    match_text = Some(captures.as_str().to_string());
                }
            } else if case_sensitive {
                if line.contains(&args.query) {
                    match_text = Some(args.query.clone());
                }
            } else if line.to_lowercase().contains(&run_query) {
                match_text = Some(args.query.clone());
            }

            let match_text = if let Some(value) = match_text {
                value
            } else {
                continue;
            };

            let start_before = index.saturating_sub(before_lines);
            let end_after = (index + 1 + after_lines).min(total_lines);

            let before_context = lines[start_before..index].to_vec();
            let after_context = if index + 1 <= end_after {
                lines[index + 1..end_after].to_vec()
            } else {
                Vec::new()
            };

            matches.push(serde_json::json!({
                "line_number": index + 1,
                "line": line,
                "match_text": match_text,
                "before": before_context,
                "after": after_context,
            }));
        }

        let truncated_flag = truncated || matches.len() >= max_matches && total_lines > 0;

        Ok(serde_json::json!({
            "operation": "search_in_file",
            "path": args.path,
            "query": args.query,
            "is_regex": is_regex,
            "case_sensitive": case_sensitive,
            "max_matches": max_matches,
            "count": matches.len(),
            "total_lines": total_lines,
            "truncated": truncated_flag,
            "before_lines": before_lines,
            "after_lines": after_lines,
            "matches": matches,
        }))
    }
}
