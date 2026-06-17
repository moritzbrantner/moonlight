use crate::types::{CommandForm, TargetCommand};
use bytes::Bytes;
use moonlight_core::{
    compare::capture_body, target::CapturedTarget, BodyCapture, TargetObservation,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, process::Stdio, time::Instant};
use tokio::{
    io::{self, AsyncRead, AsyncReadExt},
    process::Command,
    time::{timeout, Duration},
};

pub(crate) async fn run_command(
    label: &'static str,
    command: &TargetCommand,
    max_body_capture_bytes: usize,
    target_timeout_ms: u64,
) -> CapturedTarget {
    let started = Instant::now();
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return CapturedTarget {
                observation: TargetObservation {
                    status: None,
                    headers: BTreeMap::new(),
                    body: capture_body(&[], max_body_capture_bytes),
                    stderr: Some(capture_body(&[], max_body_capture_bytes)),
                    latency_ms: started.elapsed().as_millis(),
                    error: Some(format!("{label} command failed to start: {error}")),
                },
                body_bytes: Bytes::new(),
                stderr_bytes: Bytes::new(),
            };
        }
    };

    let stdout = tokio::spawn(read_optional_stream(
        child.stdout.take(),
        max_body_capture_bytes,
    ));
    let stderr = tokio::spawn(read_optional_stream(
        child.stderr.take(),
        max_body_capture_bytes,
    ));
    let wait = timeout(Duration::from_millis(target_timeout_ms), child.wait()).await;

    let status = match wait {
        Ok(status) => status,
        Err(_) => {
            let _ = child.kill().await;
            let stdout = join_stream(stdout, max_body_capture_bytes).await;
            let stderr = join_stream(stderr, max_body_capture_bytes).await;
            return CapturedTarget {
                observation: TargetObservation {
                    status: None,
                    headers: BTreeMap::new(),
                    body: stdout.capture,
                    stderr: Some(stderr.capture),
                    latency_ms: started.elapsed().as_millis(),
                    error: Some(format!(
                        "{label} command timed out after {target_timeout_ms} ms"
                    )),
                },
                body_bytes: stdout.bytes,
                stderr_bytes: stderr.bytes,
            };
        }
    };

    let stdout = match stdout.await {
        Ok(stdout) => stdout,
        Err(error) => {
            return command_read_error(
                label,
                "stdout",
                io::Error::other(error.to_string()),
                started,
                max_body_capture_bytes,
            );
        }
    };
    let stderr = match stderr.await {
        Ok(stderr) => stderr,
        Err(error) => {
            return command_read_error(
                label,
                "stderr",
                io::Error::other(error.to_string()),
                started,
                max_body_capture_bytes,
            );
        }
    };

    let stdout = match stdout {
        Ok(stdout) => stdout,
        Err(error) => {
            return command_read_error(label, "stdout", error, started, max_body_capture_bytes);
        }
    };
    let stderr = match stderr {
        Ok(stderr) => stderr,
        Err(error) => {
            return command_read_error(label, "stderr", error, started, max_body_capture_bytes);
        }
    };
    let status = match status {
        Ok(status) => status,
        Err(error) => {
            return CapturedTarget {
                observation: TargetObservation {
                    status: None,
                    headers: BTreeMap::new(),
                    body: stdout.capture,
                    stderr: Some(stderr.capture),
                    latency_ms: started.elapsed().as_millis(),
                    error: Some(format!("{label} command wait failed: {error}")),
                },
                body_bytes: stdout.bytes,
                stderr_bytes: stderr.bytes,
            };
        }
    };

    let error = status
        .code()
        .is_none()
        .then(|| format!("{label} command terminated by signal"));

    CapturedTarget {
        observation: TargetObservation {
            status: status.code().and_then(|code| u16::try_from(code).ok()),
            headers: BTreeMap::new(),
            body: stdout.capture,
            stderr: Some(stderr.capture),
            latency_ms: started.elapsed().as_millis(),
            error,
        },
        body_bytes: stdout.bytes,
        stderr_bytes: stderr.bytes,
    }
}

impl TargetCommand {
    pub(crate) fn spawn(&self) -> io::Result<tokio::process::Child> {
        let mut command = match &self.form {
            CommandForm::Shell(command) => {
                let mut process = Command::new("sh");
                process.arg("-lc").arg(command);
                process
            }
            CommandForm::Argv(argv) => {
                let mut process = Command::new(&argv[0]);
                process.args(&argv[1..]);
                process
            }
        };
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        command.envs(&self.env);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }

    pub(crate) fn display(&self) -> String {
        match &self.form {
            CommandForm::Shell(command) => command.clone(),
            CommandForm::Argv(argv) => argv
                .iter()
                .map(|arg| shell_quote(arg))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

fn command_read_error(
    label: &'static str,
    stream: &'static str,
    error: io::Error,
    started: Instant,
    max_body_capture_bytes: usize,
) -> CapturedTarget {
    CapturedTarget {
        observation: TargetObservation {
            status: None,
            headers: BTreeMap::new(),
            body: capture_body(&[], max_body_capture_bytes),
            stderr: Some(capture_body(&[], max_body_capture_bytes)),
            latency_ms: started.elapsed().as_millis(),
            error: Some(format!("{label} command failed to read {stream}: {error}")),
        },
        body_bytes: Bytes::new(),
        stderr_bytes: Bytes::new(),
    }
}

#[derive(Debug)]
struct CapturedStream {
    bytes: Bytes,
    capture: BodyCapture,
}

async fn read_optional_stream<R>(
    reader: Option<R>,
    max_body_capture_bytes: usize,
) -> io::Result<CapturedStream>
where
    R: AsyncRead + Unpin,
{
    match reader {
        Some(reader) => read_stream(reader, max_body_capture_bytes).await,
        None => Ok(CapturedStream {
            bytes: Bytes::new(),
            capture: capture_body(&[], max_body_capture_bytes),
        }),
    }
}

async fn join_stream(
    handle: tokio::task::JoinHandle<io::Result<CapturedStream>>,
    max_body_capture_bytes: usize,
) -> CapturedStream {
    match handle.await {
        Ok(Ok(stream)) => stream,
        _ => CapturedStream {
            bytes: Bytes::new(),
            capture: capture_body(&[], max_body_capture_bytes),
        },
    }
}

async fn read_stream<R>(mut reader: R, max_body_capture_bytes: usize) -> io::Result<CapturedStream>
where
    R: AsyncRead + Unpin,
{
    let mut hasher = Sha256::new();
    let mut bytes = Vec::new();
    let mut preview = Vec::with_capacity(max_body_capture_bytes.min(8192));
    let mut buffer = [0_u8; 8192];
    let mut size_bytes = 0;

    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        hasher.update(chunk);
        bytes.extend_from_slice(chunk);
        size_bytes += read;

        if preview.len() < max_body_capture_bytes {
            let remaining = max_body_capture_bytes - preview.len();
            preview.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        }
    }

    Ok(CapturedStream {
        bytes: Bytes::from(bytes),
        capture: BodyCapture {
            size_bytes,
            sha256: hex::encode(hasher.finalize()),
            preview: String::from_utf8_lossy(&preview).to_string(),
            truncated: size_bytes > max_body_capture_bytes,
        },
    })
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value.bytes().all(|byte| {
        matches!(
            byte,
            b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'_'
                | b'-'
                | b'.'
                | b'/'
                | b':'
                | b'+'
                | b','
                | b'='
        )
    }) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
