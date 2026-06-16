use crate::{
    command_form::{parse_optional_command_form, parse_required_command_form, CommandFormLabels},
    config::{build_compare_config, CliDefaults},
    types::{BatchCase, Case, PreparedCase},
};
use anyhow::Context;
use std::{path::PathBuf, sync::Arc};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, BufReader},
};

pub(crate) async fn read_batch_cases(
    input: &PathBuf,
    defaults: &CliDefaults,
) -> anyhow::Result<Vec<Case>> {
    let lines = if input.as_os_str() == "-" {
        read_stdin_lines().await?
    } else {
        fs::read_to_string(input)
            .await
            .with_context(|| format!("failed to read batch input {}", input.display()))?
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    };

    let mut cases = Vec::new();
    for (index, line) in lines.into_iter().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let case: BatchCase = serde_json::from_str(&line)
            .with_context(|| format!("invalid batch JSONL on line {line_number}"))?;
        cases.push(case_from_batch(line_number, case, defaults)?);
    }

    Ok(cases)
}

pub(crate) fn prepare_cases(cases: Vec<Case>, defaults: &CliDefaults) -> Vec<PreparedCase> {
    let default_compare_config = Arc::new(build_compare_config(
        &defaults.ignore_json_paths,
        &defaults.ignore_json_path_patterns,
        &defaults.redact_json_paths,
        &defaults.redact_json_path_patterns,
        &defaults.ignore_headers,
        defaults.ignore_stderr,
    ));
    cases
        .into_iter()
        .map(|case| {
            let compare_config = if case.uses_default_compare_config(defaults) {
                Arc::clone(&default_compare_config)
            } else {
                Arc::new(build_compare_config(
                    &case.ignore_json_paths,
                    &case.ignore_json_path_patterns,
                    &case.redact_json_paths,
                    &case.redact_json_path_patterns,
                    &case.ignore_headers,
                    case.ignore_stderr,
                ))
            };
            PreparedCase {
                case,
                compare_config,
            }
        })
        .collect()
}

fn case_from_batch(
    line_number: usize,
    case: BatchCase,
    defaults: &CliDefaults,
) -> anyhow::Result<Case> {
    let mut ignore_json_paths = defaults.ignore_json_paths.clone();
    ignore_json_paths.extend(case.ignore_json_paths);
    let mut ignore_json_path_patterns = defaults.ignore_json_path_patterns.clone();
    ignore_json_path_patterns.extend(case.ignore_json_path_patterns);
    let mut redact_json_paths = defaults.redact_json_paths.clone();
    redact_json_paths.extend(case.redact_json_paths);
    let mut redact_json_path_patterns = defaults.redact_json_path_patterns.clone();
    redact_json_path_patterns.extend(case.redact_json_path_patterns);
    let mut ignore_headers = defaults.ignore_headers.clone();
    ignore_headers.extend(case.ignore_headers);
    let target_timeout_ms = case
        .target_timeout_ms
        .map(moonlight_core::config::normalize_timeout)
        .unwrap_or(defaults.target_timeout_ms);

    Ok(Case {
        primary: parse_required_command_form(
            batch_labels("primary"),
            case.primary,
            case.primary_argv,
        )
        .with_context(|| format!("invalid batch JSONL on line {line_number}"))?,
        candidate: parse_required_command_form(
            batch_labels("candidate"),
            case.candidate,
            case.candidate_argv,
        )
        .with_context(|| format!("invalid batch JSONL on line {line_number}"))?,
        secondary: parse_optional_command_form(
            batch_labels("secondary"),
            case.secondary,
            case.secondary_argv,
        )
        .with_context(|| format!("invalid batch JSONL on line {line_number}"))?,
        max_body_capture_bytes: case
            .max_body_capture_bytes
            .unwrap_or(defaults.max_body_capture_bytes),
        ignore_json_paths,
        ignore_json_path_patterns,
        redact_json_paths,
        redact_json_path_patterns,
        ignore_headers,
        ignore_stderr: defaults.ignore_stderr || case.ignore_stderr,
        target_timeout_ms,
    })
}

fn batch_labels(role: &'static str) -> CommandFormLabels {
    let (shell, argv) = match role {
        "primary" => ("primary", "primary_argv"),
        "candidate" => ("candidate", "candidate_argv"),
        "secondary" => ("secondary", "secondary_argv"),
        _ => unreachable!("unknown target role"),
    };

    CommandFormLabels {
        role,
        shell,
        argv,
        reject_empty_shell: true,
    }
}

async fn read_stdin_lines() -> anyhow::Result<Vec<String>> {
    let mut lines = BufReader::new(io::stdin()).lines();
    let mut output = Vec::new();
    while let Some(line) = lines.next_line().await? {
        output.push(line);
    }
    Ok(output)
}
