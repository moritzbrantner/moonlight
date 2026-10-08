use crate::types::{CommandForm, TargetCommand};
use bytes::Bytes;
use moonlight_core::{
    compare::{capture_body, capture_body_with_redaction_patterns},
    target::CapturedTarget,
    BodyCapture, TargetObservation,
};
use std::{collections::BTreeMap, process::Stdio, time::Instant};
use tokio::{
    io::{self, AsyncRead, AsyncReadExt},
    process::{Child, Command},
    time::{timeout_at, Duration, Instant as TokioInstant},
};

pub(crate) async fn run_command(
    label: &'static str,
    command: &TargetCommand,
    max_body_capture_bytes: usize,
    target_timeout_ms: u64,
) -> CapturedTarget {
    run_command_with_redactions(
        label,
        command,
        max_body_capture_bytes,
        target_timeout_ms,
        &[],
        &[],
    )
    .await
}

pub(crate) async fn run_command_with_redactions(
    label: &'static str,
    command: &TargetCommand,
    max_body_capture_bytes: usize,
    target_timeout_ms: u64,
    redact_json_paths: &[String],
    redact_json_path_patterns: &[String],
) -> CapturedTarget {
    let started = Instant::now();
    let deadline = TokioInstant::now() + Duration::from_millis(target_timeout_ms);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return error_target(
                label,
                format!("{label} command failed to start: {error}"),
                started,
                max_body_capture_bytes,
            );
        }
    };
    let process_group_id = child.id();

    let mut stdout = tokio::spawn(read_optional_stream(child.stdout.take()));
    let mut stderr = tokio::spawn(read_optional_stream(child.stderr.take()));

    let status = match timeout_at(deadline, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            terminate_process_tree(&mut child, process_group_id);
            stdout.abort();
            stderr.abort();
            return error_target(
                label,
                format!("{label} command wait failed: {error}"),
                started,
                max_body_capture_bytes,
            );
        }
        Err(_) => {
            terminate_process_tree(&mut child, process_group_id);
            stdout.abort();
            stderr.abort();
            return timeout_target(label, started, max_body_capture_bytes, target_timeout_ms);
        }
    };

    let streams = timeout_at(deadline, async {
        let stdout_result = (&mut stdout).await;
        let stderr_result = (&mut stderr).await;
        (stdout_result, stderr_result)
    })
    .await;

    let (stdout_bytes, stderr_bytes) = match streams {
        Ok((stdout_result, stderr_result)) => {
            let stdout_bytes = match join_stream_result(stdout_result) {
                Ok(bytes) => bytes,
                Err(error) => {
                    terminate_process_tree(&mut child, process_group_id);
                    stderr.abort();
                    return command_read_error(
                        label,
                        "stdout",
                        error,
                        started,
                        max_body_capture_bytes,
                    );
                }
            };
            let stderr_bytes = match join_stream_result(stderr_result) {
                Ok(bytes) => bytes,
                Err(error) => {
                    terminate_process_tree(&mut child, process_group_id);
                    return command_read_error(
                        label,
                        "stderr",
                        error,
                        started,
                        max_body_capture_bytes,
                    );
                }
            };
            (stdout_bytes, stderr_bytes)
        }
        Err(_) => {
            // A descendant can outlive the direct child while retaining an inherited
            // stdout/stderr pipe. The lifecycle deadline covers that drain as well.
            terminate_process_tree(&mut child, process_group_id);
            stdout.abort();
            stderr.abort();
            return timeout_target(label, started, max_body_capture_bytes, target_timeout_ms);
        }
    };

    let error = status
        .code()
        .is_none()
        .then(|| format!("{label} command terminated by signal"));

    captured_target(
        status.code().and_then(|code| u16::try_from(code).ok()),
        stdout_bytes,
        stderr_bytes,
        started,
        error,
        CapturePolicy {
            max_body_capture_bytes,
            redact_json_paths,
            redact_json_path_patterns,
        },
    )
}

impl TargetCommand {
    pub(crate) fn spawn(&self) -> io::Result<Child> {
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
            .stderr(Stdio::piped());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.as_std_mut().process_group(0);
        }

        command.spawn()
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

#[derive(Clone, Copy)]
struct CapturePolicy<'a> {
    max_body_capture_bytes: usize,
    redact_json_paths: &'a [String],
    redact_json_path_patterns: &'a [String],
}

impl CapturePolicy<'_> {
    fn capture(self, bytes: &[u8]) -> BodyCapture {
        capture_body_with_redaction_patterns(
            bytes,
            self.max_body_capture_bytes,
            self.redact_json_paths,
            self.redact_json_path_patterns,
        )
    }
}

fn captured_target(
    status: Option<u16>,
    body_bytes: Bytes,
    stderr_bytes: Bytes,
    started: Instant,
    error: Option<String>,
    capture_policy: CapturePolicy<'_>,
) -> CapturedTarget {
    CapturedTarget {
        observation: TargetObservation {
            status,
            headers: BTreeMap::new(),
            body: capture_policy.capture(&body_bytes),
            stderr: Some(capture_policy.capture(&stderr_bytes)),
            latency_ms: started.elapsed().as_millis(),
            error,
        },
        transport_headers: Default::default(),
        body_bytes,
        stderr_bytes,
    }
}

fn command_read_error(
    label: &'static str,
    stream: &'static str,
    error: io::Error,
    started: Instant,
    max_body_capture_bytes: usize,
) -> CapturedTarget {
    error_target(
        label,
        format!("{label} command failed to read {stream}: {error}"),
        started,
        max_body_capture_bytes,
    )
}

fn timeout_target(
    label: &'static str,
    started: Instant,
    max_body_capture_bytes: usize,
    target_timeout_ms: u64,
) -> CapturedTarget {
    // Partial output is intentionally discarded on timeout. Retaining it would
    // require waiting for untrusted descendants and would violate the lifecycle
    // deadline that the timeout promises.
    error_target(
        label,
        format!("{label} command timed out after {target_timeout_ms} ms"),
        started,
        max_body_capture_bytes,
    )
}

fn error_target(
    _label: &'static str,
    error: String,
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
            error: Some(error),
        },
        transport_headers: Default::default(),
        body_bytes: Bytes::new(),
        stderr_bytes: Bytes::new(),
    }
}

async fn read_optional_stream<R>(reader: Option<R>) -> io::Result<Bytes>
where
    R: AsyncRead + Unpin,
{
    match reader {
        Some(reader) => read_stream(reader).await,
        None => Ok(Bytes::new()),
    }
}

fn join_stream_result(
    result: Result<io::Result<Bytes>, tokio::task::JoinError>,
) -> io::Result<Bytes> {
    match result {
        Ok(result) => result,
        Err(error) => Err(io::Error::other(error.to_string())),
    }
}

async fn read_stream<R>(mut reader: R) -> io::Result<Bytes>
where
    R: AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await?;
    Ok(Bytes::from(bytes))
}

fn terminate_process_tree(child: &mut Child, process_group_id: Option<u32>) {
    #[cfg(unix)]
    if let Some(process_group_id) = process_group_id {
        // Each target starts in a fresh process group. Killing the group also
        // terminates descendants that inherited the target's output pipes.
        let group = format!("-{process_group_id}");
        let _ = std::process::Command::new("kill")
            .args(["-KILL", "--", &group])
            .status();
    }

    #[cfg(windows)]
    if let Some(process_group_id) = process_group_id {
        let pid = process_group_id.to_string();
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid, "/T", "/F"])
            .status();
    }

    let _ = child.start_kill();
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
