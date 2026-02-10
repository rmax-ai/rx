use rig::streaming::StreamingPrompt;
use futures::StreamExt;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use serde::{Deserialize, Serialize};

use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai;
use rig::tool::Tool;
use rig::completion::ToolDefinition;

#[derive(Deserialize, Serialize)]
struct GreetingsArgs {
    name: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Greetings error")]
struct GreetingsError;

struct Greetings;

impl Tool for Greetings {
    const NAME: &'static str = "greetings";
    type Args = GreetingsArgs;
    type Output = String;
    type Error = GreetingsError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "greetings".to_string(),
            description: "Greet a person by name".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The name of the person to greet"
                    }
                },
                "required": ["name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Hello, {}! How can I help you today?", args.name))
    }
}

#[derive(Deserialize, Serialize)]
struct CalculatorArgs {
    x: f64,
    y: f64,
    op: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Calculator error")]
struct CalculatorError;

struct Calculator;

impl Tool for Calculator {
    const NAME: &'static str = "calculator";
    type Args = CalculatorArgs;
    type Output = String;
    type Error = CalculatorError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "calculator".to_string(),
            description: "Perform basic arithmetic operations".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "op": {
                        "type": "string",
                        "enum": ["add", "subtract", "multiply", "divide"]
                    }
                },
                "required": ["x", "y", "op"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = match args.op.as_str() {
            "add" => args.x + args.y,
            "subtract" => args.x - args.y,
            "multiply" => args.x * args.y,
            "divide" => args.x / args.y,
            _ => return Err(CalculatorError),
        };
        Ok(result.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize structured logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let client = openai::Client::from_env();
    let agent = client
        .agent(openai::GPT_5_MINI)
        .tool(Greetings)
        .tool(Calculator)
        .build();

    let mut stream = agent
        .stream_prompt("What is 123.45 multiplied by 67.89? And also say hello to Rmax.")
        .await;

    while let Some(chunk) = stream.next().await {
        info!(chunk = ?chunk, "Received chunk");
    }

    Ok(())
}
