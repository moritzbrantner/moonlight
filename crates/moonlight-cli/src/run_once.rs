use crate::{
    args::RunArgs,
    command_form::{
        parse_json_argv_flag, parse_optional_command_form, parse_required_command_form,
        CommandFormLabels,
    },
    config::build_compare_config,
    execute::execute_case,
    types::{Case, PreparedCase},
};
use moonlight_core::storage::RunWriter;
use std::sync::Arc;

pub(crate) async fn run(args: RunArgs) -> anyhow::Result<()> {
    let case = Case {
        primary: parse_required_command_form(
            run_labels("primary"),
            args.primary,
            parse_json_argv_flag("--primary-argv", args.primary_argv)?,
        )?,
        candidate: parse_required_command_form(
            run_labels("candidate"),
            args.candidate,
            parse_json_argv_flag("--candidate-argv", args.candidate_argv)?,
        )?,
        secondary: parse_optional_command_form(
            run_labels("secondary"),
            args.secondary,
            parse_json_argv_flag("--secondary-argv", args.secondary_argv)?,
        )?,
        max_body_capture_bytes: args.max_body_capture_bytes,
        ignored_json_paths: args.ignored_json_paths,
        ignored_headers: args.ignored_headers,
        ignore_stderr: args.ignore_stderr,
    };
    let compare_config = Arc::new(build_compare_config(
        &case.ignored_json_paths,
        &case.ignored_headers,
        case.ignore_stderr,
    ));
    let run = execute_case(
        PreparedCase {
            case,
            compare_config,
        },
        args.serial_targets,
    )
    .await;

    let writer = RunWriter::open(args.storage.storage_path).await?;
    writer.append(&run).await?;
    writer.flush().await?;

    if !args.quiet {
        if args.compact {
            println!("{}", serde_json::to_string(&run)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&run)?);
        }
    }
    Ok(())
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
