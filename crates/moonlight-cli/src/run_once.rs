use crate::{
    args::RunArgs,
    command_form::{
        parse_json_argv_flag, parse_optional_command_form, parse_required_command_form,
        CommandFormLabels,
    },
    config::{build_compare_config, CliDefaults},
    execute::execute_case,
    types::{Case, PreparedCase},
};
use moonlight_core::config::normalize_timeout;
use moonlight_core::config::CliTargetConfig;
use moonlight_core::storage::RunWriter;
use std::sync::Arc;

pub(crate) async fn run(args: RunArgs, defaults: &CliDefaults) -> anyhow::Result<()> {
    let targets = merge_targets(&args, &defaults.run.targets)?;
    let mut ignore_json_paths = defaults.ignore_json_paths.clone();
    ignore_json_paths.extend(args.ignore_json_paths);
    let mut ignore_json_path_patterns = defaults.ignore_json_path_patterns.clone();
    ignore_json_path_patterns.extend(args.ignore_json_path_patterns);
    let mut redact_json_paths = defaults.redact_json_paths.clone();
    redact_json_paths.extend(args.redact_json_paths);
    let mut redact_json_path_patterns = defaults.redact_json_path_patterns.clone();
    redact_json_path_patterns.extend(args.redact_json_path_patterns);
    let mut ignore_headers = defaults.ignore_headers.clone();
    ignore_headers.extend(args.ignore_headers);
    let ignore_stderr = defaults.ignore_stderr || args.ignore_stderr;
    let target_timeout_ms = args
        .target_timeout_ms
        .map(normalize_timeout)
        .unwrap_or(defaults.target_timeout_ms);
    let max_body_capture_bytes = args
        .max_body_capture_bytes
        .unwrap_or(defaults.max_body_capture_bytes);
    let serial_targets = defaults.run.serial_targets || args.serial_targets;
    let quiet = defaults.run.quiet || args.quiet;
    let compact = defaults.run.compact || args.compact;
    let storage_path = args
        .storage
        .storage_path
        .unwrap_or_else(|| defaults.storage_path.clone());

    let case = Case {
        primary: parse_required_command_form(
            run_labels("primary"),
            targets.primary,
            targets.primary_argv,
        )?,
        candidate: parse_required_command_form(
            run_labels("candidate"),
            targets.candidate,
            targets.candidate_argv,
        )?,
        secondary: parse_optional_command_form(
            run_labels("secondary"),
            targets.secondary,
            targets.secondary_argv,
        )?,
        max_body_capture_bytes,
        ignore_json_paths,
        ignore_json_path_patterns,
        redact_json_paths,
        redact_json_path_patterns,
        ignore_headers,
        ignore_stderr,
        target_timeout_ms,
    };
    let compare_config = Arc::new(build_compare_config(
        &case.ignore_json_paths,
        &case.ignore_json_path_patterns,
        &case.redact_json_paths,
        &case.redact_json_path_patterns,
        &case.ignore_headers,
        case.ignore_stderr,
    ));
    let run = execute_case(
        PreparedCase {
            case,
            compare_config,
        },
        serial_targets,
    )
    .await;

    let writer = RunWriter::open(storage_path).await?;
    writer.append(&run).await?;
    writer.flush().await?;

    if !quiet {
        if compact {
            println!("{}", serde_json::to_string(&run)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&run)?);
        }
    }
    Ok(())
}

fn merge_targets(args: &RunArgs, defaults: &CliTargetConfig) -> anyhow::Result<CliTargetConfig> {
    Ok(CliTargetConfig {
        primary: merge_shell(&args.primary, &args.primary_argv, &defaults.primary),
        primary_argv: merge_argv(
            "--primary-argv",
            &args.primary,
            &args.primary_argv,
            &defaults.primary_argv,
        )?,
        candidate: merge_shell(&args.candidate, &args.candidate_argv, &defaults.candidate),
        candidate_argv: merge_argv(
            "--candidate-argv",
            &args.candidate,
            &args.candidate_argv,
            &defaults.candidate_argv,
        )?,
        secondary: merge_shell(&args.secondary, &args.secondary_argv, &defaults.secondary),
        secondary_argv: merge_argv(
            "--secondary-argv",
            &args.secondary,
            &args.secondary_argv,
            &defaults.secondary_argv,
        )?,
    })
}

fn merge_shell(
    arg_shell: &Option<String>,
    arg_argv: &Option<String>,
    default_shell: &Option<String>,
) -> Option<String> {
    if arg_shell.is_some() || arg_argv.is_some() {
        arg_shell.clone()
    } else {
        default_shell.clone()
    }
}

fn merge_argv(
    label: &'static str,
    arg_shell: &Option<String>,
    arg_argv: &Option<String>,
    default_argv: &Option<Vec<String>>,
) -> anyhow::Result<Option<Vec<String>>> {
    if arg_shell.is_some() || arg_argv.is_some() {
        parse_json_argv_flag(label, arg_argv.clone())
    } else {
        Ok(default_argv.clone())
    }
}

fn run_labels(role: &'static str) -> CommandFormLabels {
    let (shell, argv) = match role {
        "primary" => ("--primary", "--primary-argv"),
        "candidate" => ("--candidate", "--candidate-argv"),
        "secondary" => ("--secondary", "--secondary-argv"),
        _ => unreachable!("unknown target role"),
    };

    CommandFormLabels {
        role,
        shell,
        argv,
        reject_empty_shell: false,
    }
}
