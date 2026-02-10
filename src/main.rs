use rig::streaming::StreamingPrompt;
use futures::StreamExt;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai;

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
        .build();

    let mut stream = agent
        .stream_prompt("Write a haiku about Rust")
        .await;

    while let Some(chunk) = stream.next().await {
        info!(chunk = ?chunk, "Received chunk");
    }

    Ok(())
}
