use crate::types::TargetCommand;
use bytes::Bytes;
use moonlight_core::{
    compare::{capture_body, CapturedTarget},
    BodyCapture, TargetObservation,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, process::Stdio, time::Instant};
use tokio::{
    io::{self, AsyncRead, AsyncReadExt},
    process::Command,
};

pub(crate) async fn run_command(
    label: &'static str,
    command: &TargetCommand,
    max_body_capture_bytes: usize,
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

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (stdout, stderr, status) = tokio::join!(
        read_optional_stream(stdout, max_body_capture_bytes),
        read_optional_stream(stderr, max_body_capture_bytes),
        child.wait(),
    );

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
        let mut command = match self {
            Self::Shell(command) => {
                let mut process = Command::new("sh");
                process.arg("-lc").arg(command);
                process
            }
            Self::Argv(argv) => {
                let mut process = Command::new(&argv[0]);
                process.args(&argv[1..]);
                process
            }
        };
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }

    pub(crate) fn display(&self) -> String {
        match self {
            Self::Shell(command) => command.clone(),
            Self::Argv(argv) => argv
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
