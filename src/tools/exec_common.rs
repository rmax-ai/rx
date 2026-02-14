use anyhow::Result;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Command, Stdio};
use tokio::task::JoinHandle;
use tokio::time::timeout;

pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
pub const DEFAULT_MAX_STDOUT_BYTES: usize = 32 * 1024;
pub const DEFAULT_MAX_STDERR_BYTES: usize = 16 * 1024;
pub const STATUS_STDERR_BYTES: usize = 1024;

#[derive(Debug)]
pub struct ExecCommandRequest {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
    pub stdin: Option<String>,
}

#[derive(Debug)]
pub struct ExecCommandResult {
    pub exit_code: Option<i32>,
    pub success: bool,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

pub async fn execute_command(request: ExecCommandRequest) -> Result<ExecCommandResult> {
    let ExecCommandRequest {
        command,
        args,
        cwd,
        timeout_seconds,
        capture_stdout,
        capture_stderr,
        max_stdout_bytes,
        max_stderr_bytes,
        stdin,
    } = request;

    let mut cmd = Command::new(&command);
    cmd.args(&args);

    if let Some(cwd) = cwd.as_deref() {
        cmd.current_dir(Path::new(cwd));
    }

    if capture_stdout {
        cmd.stdout(Stdio::piped());
    } else {
        cmd.stdout(Stdio::null());
    }

    if capture_stderr {
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stderr(Stdio::null());
    }

    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    let mut child = cmd.spawn()?;

    if let Some(input) = stdin {
        if let Some(mut writer) = child.stdin.take() {
            tokio::spawn(async move {
                let _ = writer.write_all(input.as_bytes()).await;
                let _ = writer.shutdown().await;
            });
        }
    }

    let stdout_handle = if capture_stdout {
        child
            .stdout
            .take()
            .map(|stdout| tokio::spawn(read_stream_limited(stdout, max_stdout_bytes)))
    } else {
        None
    };

    let stderr_handle = if capture_stderr {
        child
            .stderr
            .take()
            .map(|stderr| tokio::spawn(read_stream_limited(stderr, max_stderr_bytes)))
    } else {
        None
    };

    let duration = Duration::from_secs(timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS));
    let start = Instant::now();
    let wait_result = timeout(duration, child.wait()).await;
    let timed_out = wait_result.is_err();
    let exit_status = if timed_out {
        let _ = child.kill().await;
        child.wait().await.ok()
    } else {
        wait_result
            .unwrap()
            .map_err(|err| err.into())
            .ok()
            .flatten()
    };
    let duration_ms = match start.elapsed().as_millis().try_into() {
        Ok(ms) => ms,
        Err(_) => u64::MAX,
    };

    let success = exit_status
        .as_ref()
        .map(|status| status.success())
        .unwrap_or(false);
    let exit_code = exit_status.and_then(|status| status.code());

    let (stdout_bytes, stdout_truncated) = capture_stream(stdout_handle).await?;
    let (stderr_bytes, stderr_truncated) = capture_stream(stderr_handle).await?;

    let stdout_text = stdout_bytes.map(|bytes| String::from_utf8_lossy(&bytes).to_string());
    let stderr_text = stderr_bytes.map(|bytes| String::from_utf8_lossy(&bytes).to_string());

    Ok(ExecCommandResult {
        exit_code,
        success,
        timed_out,
        duration_ms,
        stdout: stdout_text,
        stderr: stderr_text,
        stdout_truncated,
        stderr_truncated,
    })
}

async fn capture_stream(
    handle: Option<JoinHandle<io::Result<(Vec<u8>, bool)>>>,
) -> io::Result<(Option<Vec<u8>>, bool)> {
    if let Some(handle) = handle {
        match handle.await {
            Ok(Ok((bytes, truncated))) => Ok((Some(bytes), truncated)),
            Ok(Err(err)) => Err(err),
            Err(join_err) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("capture task failed: {}", join_err),
            )),
        }
    } else {
        Ok((None, false))
    }
}

async fn read_stream_limited<R>(mut reader: R, max_bytes: usize) -> io::Result<(Vec<u8>, bool)>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut buffer = Vec::with_capacity(max_bytes.min(8192));
    let mut truncated = false;
    let mut chunk = [0u8; 8192];

    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }

        if buffer.len() < max_bytes {
            let remaining = max_bytes - buffer.len();
            if read <= remaining {
                buffer.extend_from_slice(&chunk[..read]);
            } else {
                buffer.extend_from_slice(&chunk[..remaining]);
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }

    Ok((buffer, truncated))
}
