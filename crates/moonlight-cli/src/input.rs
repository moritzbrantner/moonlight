use crate::{
    config::{build_compare_config, DEFAULT_MAX_BODY_CAPTURE_BYTES},
    types::{BatchCase, Case, PreparedCase, TargetCommand},
};
use anyhow::{bail, Context};
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
            line_number,
            "primary",
            case.primary,
            case.primary_argv,
        )?,
        candidate: parse_required_command_form(
            line_number,
            "candidate",
            case.candidate,
            case.candidate_argv,
        )?,
        secondary: parse_optional_command_form(
            line_number,
            "secondary",
            case.secondary,
            case.secondary_argv,
        )?,
        max_body_capture_bytes: case
            .max_body_capture_bytes
            .unwrap_or(DEFAULT_MAX_BODY_CAPTURE_BYTES),
        ignored_json_paths: case.ignored_json_paths,
        ignored_headers: case.ignored_headers,
        ignore_stderr: case.ignore_stderr,
    })
}

fn parse_required_command_form(
    line_number: usize,
    role: &str,
    shell: Option<String>,
    argv: Option<Vec<String>>,
) -> anyhow::Result<TargetCommand> {
    parse_command_form(line_number, role, shell, argv)?.with_context(|| {
        format!("invalid batch JSONL on line {line_number}: {role} command form is required")
    })
}

fn parse_optional_command_form(
    line_number: usize,
    role: &str,
    shell: Option<String>,
    argv: Option<Vec<String>>,
) -> anyhow::Result<Option<TargetCommand>> {
    parse_command_form(line_number, role, shell, argv)
}

fn parse_command_form(
    line_number: usize,
    role: &str,
    shell: Option<String>,
    argv: Option<Vec<String>>,
) -> anyhow::Result<Option<TargetCommand>> {
    match (shell, argv) {
        (Some(_), Some(_)) => bail!(
            "invalid batch JSONL on line {line_number}: provide exactly one of {role} or {role}_argv"
        ),
        (Some(command), None) => {
            if command.trim().is_empty() {
                bail!("invalid batch JSONL on line {line_number}: {role} must not be empty");
            }
            Ok(Some(TargetCommand::Shell(command)))
        }
        (None, Some(argv)) => {
            if argv.is_empty() {
                bail!("invalid batch JSONL on line {line_number}: {role}_argv must not be empty");
            }
            if argv[0].trim().is_empty() {
                bail!(
                    "invalid batch JSONL on line {line_number}: {role}_argv command must not be empty"
                );
            }
            Ok(Some(TargetCommand::Argv(argv)))
        }
        (None, None) => Ok(None),
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
