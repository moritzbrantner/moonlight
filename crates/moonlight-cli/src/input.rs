use crate::{
    command_form::{parse_optional_command_form, parse_required_command_form, CommandFormLabels},
    config::{build_compare_config, DEFAULT_MAX_BODY_CAPTURE_BYTES},
    types::{BatchCase, Case, PreparedCase},
};
use anyhow::Context;
use std::{path::PathBuf, sync::Arc};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, BufReader},
};

pub(crate) async fn read_batch_cases(input: &PathBuf) -> anyhow::Result<Vec<Case>> {
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
        cases.push(case_from_batch(line_number, case)?);
    }

    Ok(cases)
}

pub(crate) fn prepare_cases(cases: Vec<Case>) -> Vec<PreparedCase> {
    let default_compare_config = Arc::new(build_compare_config(&[], &[], false));
    cases
        .into_iter()
        .map(|case| {
            let compare_config = if case.uses_default_compare_config() {
                Arc::clone(&default_compare_config)
            } else {
                Arc::new(build_compare_config(
                    &case.ignored_json_paths,
                    &case.ignored_headers,
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

fn case_from_batch(line_number: usize, case: BatchCase) -> anyhow::Result<Case> {
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
            .unwrap_or(DEFAULT_MAX_BODY_CAPTURE_BYTES),
        ignored_json_paths: case.ignored_json_paths,
        ignored_headers: case.ignored_headers,
        ignore_stderr: case.ignore_stderr,
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
