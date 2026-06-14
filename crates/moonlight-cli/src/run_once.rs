use crate::{
    args::RunArgs,
    config::build_compare_config,
    execute::execute_case,
    types::{Case, PreparedCase, TargetCommand},
};
use moonlight_core::storage::RunWriter;
use std::sync::Arc;

pub(crate) async fn run(args: RunArgs) -> anyhow::Result<()> {
    let case = Case {
        primary: TargetCommand::Shell(args.primary),
        candidate: TargetCommand::Shell(args.candidate),
        secondary: args.secondary.map(TargetCommand::Shell),
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
