use futures::StreamExt;
use rig::streaming::StreamingPrompt;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use std::path::PathBuf;

use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai;
use rig::tool::Tool;
use rig::completion::ToolDefinition;

#[derive(Debug, thiserror::Error)]
enum FsToolError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("command denied: {0}")]
    Denied(String),
    #[error("path denied: {0}")]
    DeniedPath(String),
    #[error("command timed out")]
    Timeout,
    #[error("command failed: {0}")]
    CommandFailed(String),
}

async fn resolve_safe_path(path: &str) -> Result<PathBuf, FsToolError> {
    let root = std::env::current_dir()?;
    let resolved = tokio::fs::canonicalize(path).await?;
    if !resolved.starts_with(&root) {
        return Err(FsToolError::DeniedPath(path.to_string()));
    }
    Ok(resolved)
}

// list_files: return directory entries (one per line)
#[derive(Deserialize, Serialize)]
struct ListFilesArgs {
    path: String,
    limit: Option<usize>,
}

struct ListFiles;

impl Tool for ListFiles {
    const NAME: &'static str = "list_files";
    type Args = ListFilesArgs;
    type Output = String;
    type Error = FsToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_files".to_string(),
            description: "List files and folders at a path (must be under the current directory). Cap results to 5000. Examples: {\"path\": \"src\"} or {\"path\": \"src\", \"limit\": 20}.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory to list"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum entries to return (cap 5000)."
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let safe_path = resolve_safe_path(&args.path).await?;
        let mut entries = fs::read_dir(&safe_path).await?;
        let mut names = Vec::new();
        let limit = args.limit.unwrap_or(5000).min(5000);
        while let Some(entry) = entries.next_entry().await? {
            names.push(entry.file_name().to_string_lossy().into_owned());
            if names.len() >= limit {
                break;
            }
        }
        names.sort();
        Ok(names.join("\n"))
    }
}

// read_file: return entire file contents
#[derive(Deserialize, Serialize)]
struct ReadFileArgs {
    path: String,
    offset: u64,
    limit: usize,
}

struct ReadFile;

impl Tool for ReadFile {
    const NAME: &'static str = "read_file";
    type Args = ReadFileArgs;
    type Output = String;
    type Error = FsToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read a slice of a file with byte offset and limit (cap 10000 bytes). Path must be under the current directory. Examples: {\"path\": \"src/main.rs\", \"offset\": 0, \"limit\": 200} or {\"path\": \"README.md\", \"offset\": 100, \"limit\": 500}.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path of the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Byte offset to start reading from"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum bytes to read (cap 10000)"
                    }
                },
                "required": ["path", "offset", "limit"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let safe_path = resolve_safe_path(&args.path).await?;
        let mut file = fs::File::open(&safe_path).await?;
        file.seek(SeekFrom::Start(args.offset)).await?;

        let max_bytes = args.limit.min(10_000) as usize;
        let mut buffer = vec![0u8; max_bytes];
        let read_len = file.read(&mut buffer).await?;
        buffer.truncate(read_len);

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }
}

#[derive(Deserialize, Serialize)]
struct BashArgs {
    command: String,
    timeout_secs: Option<u64>,
}

struct BashCommand;

impl Tool for BashCommand {
    const NAME: &'static str = "bash";
    type Args = BashArgs;
    type Output = String;
    type Error = FsToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "bash".to_string(),
            description: "Run a bash command with safeguards (timeout, deny dangerous ops). Examples: {\"command\": \"pwd\"}, {\"command\": \"ls -la src\", \"timeout_secs\": 5}. Denied: destructive commands (rm, sudo, chmod, chown, mount, etc.).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to run in bash -lc"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Optional timeout seconds (max 10)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let forbidden = [
            "rm ", "rm-", "sudo", "chmod", "chown", "chgrp", "mv ", "dd ",
            "mkfs", "mount", "umount", "shutdown", "reboot", "init ", "halt",
        ];

        let cmd_lower = args.command.to_lowercase();
        if forbidden.iter().any(|pat| cmd_lower.contains(pat)) {
            return Err(FsToolError::Denied(args.command));
        }

        let secs = args.timeout_secs.unwrap_or(5).min(10);
        let child = Command::new("bash")
            .arg("-lc")
            .arg(&args.command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let output = timeout(Duration::from_secs(secs), async {
            child.wait_with_output().await
        })
        .await
        .map_err(|_| FsToolError::Timeout)??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(FsToolError::CommandFailed(stderr));
        }

        let mut stdout = output.stdout;
        if stdout.len() > 8000 {
            stdout.truncate(8000);
        }

        Ok(String::from_utf8_lossy(&stdout).to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Read prompt from CLI
    let prompt = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if prompt.trim().is_empty() {
        eprintln!("Usage: rx <prompt>");
        std::process::exit(1);
    }

    // Structured logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let client = openai::Client::from_env();
    let agent = client
        .agent(openai::GPT_5_MINI)
        .tool(ListFiles)
        .tool(ReadFile)
        .tool(BashCommand)
        .build();

    let mut stream = agent
        .stream_prompt(prompt)
        .await;

    while let Some(chunk) = stream.next().await {
        info!(chunk = ?chunk, "Received chunk");
    }

    Ok(())
}
