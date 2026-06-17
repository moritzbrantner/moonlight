use crate::eval_config::CheckConfig;
use anyhow::Context;
use bytes::Bytes;
use moonlight_core::compare::{capture_body, CapturedTarget};
use regex::Regex;

pub(super) struct PreparedEvalCheck {
    pub(super) config: CheckConfig,
    stdout_patterns: Vec<Regex>,
    stderr_patterns: Vec<Regex>,
}

pub(super) fn prepare_checks(checks: Vec<CheckConfig>) -> anyhow::Result<Vec<PreparedEvalCheck>> {
    checks.into_iter().map(PreparedEvalCheck::new).collect()
}

impl PreparedEvalCheck {
    fn new(config: CheckConfig) -> anyhow::Result<Self> {
        let stdout_patterns = compile_patterns(
            &config.normalize_stdout_patterns,
            &format!("invalid normalize stdout pattern in check {}", config.id),
        )?;
        let stderr_patterns = compile_patterns(
            &config.normalize_stderr_patterns,
            &format!("invalid normalize stderr pattern in check {}", config.id),
        )?;

        Ok(Self {
            config,
            stdout_patterns,
            stderr_patterns,
        })
    }
}

pub(super) fn normalize_target(
    target: &mut CapturedTarget,
    check: &PreparedEvalCheck,
    max_body_capture_bytes: usize,
) -> anyhow::Result<()> {
    if check.config.ignore_stdout {
        replace_stdout(target, Bytes::new(), max_body_capture_bytes);
    } else if !check.stdout_patterns.is_empty() {
        let bytes = normalize_bytes(&target.body_bytes, &check.stdout_patterns);
        replace_stdout(target, bytes, max_body_capture_bytes);
    }

    if let Some(stderr) = &target.observation.stderr {
        if !check.stderr_patterns.is_empty() {
            let bytes = normalize_bytes(&target.stderr_bytes, &check.stderr_patterns);
            target.stderr_bytes = bytes.clone();
            target.observation.stderr = Some(capture_body(&bytes, max_body_capture_bytes));
        } else {
            target.observation.stderr = Some(stderr.clone());
        }
    }
    Ok(())
}

fn replace_stdout(target: &mut CapturedTarget, bytes: Bytes, max_body_capture_bytes: usize) {
    target.body_bytes = bytes.clone();
    target.observation.body = capture_body(&bytes, max_body_capture_bytes);
}

fn normalize_bytes(bytes: &Bytes, patterns: &[Regex]) -> Bytes {
    let mut value = String::from_utf8_lossy(bytes).to_string();
    for pattern in patterns {
        value = pattern.replace_all(&value, "<normalized>").to_string();
    }
    Bytes::from(value)
}

fn compile_patterns(patterns: &[String], context: &str) -> anyhow::Result<Vec<Regex>> {
    patterns
        .iter()
        .map(|pattern| Regex::new(pattern).with_context(|| context.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_replaces_matching_output() {
        let patterns = compile_patterns(
            &["[0-9]+ms".to_string(), "/tmp/[^ ]+".to_string()],
            "test pattern",
        )
        .unwrap();
        let output = normalize_bytes(&Bytes::from("built in 123ms at /tmp/demo"), &patterns);

        assert_eq!(
            String::from_utf8(output.to_vec()).unwrap(),
            "built in <normalized> at <normalized>"
        );
    }
}
