use crate::config::DEFAULT_MAX_BODY_CAPTURE_BYTES;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(about = "Compare command output with Moonlight's shared comparison engine")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CliCommand {
    Run(RunArgs),
    Batch(BatchArgs),
    List(StorageArgs),
    Stats(StorageArgs),
    Show(ShowArgs),
}

#[derive(Debug, Args)]
pub(crate) struct StorageArgs {
    #[arg(long, default_value = "data/moonlight/cli-runs.jsonl")]
    pub(crate) storage_path: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct ShowArgs {
    pub(crate) id: Uuid,

    #[command(flatten)]
    pub(crate) storage: StorageArgs,
}

#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(long)]
    pub(crate) primary: Option<String>,

    #[arg(long)]
    pub(crate) primary_argv: Option<String>,

    #[arg(long)]
    pub(crate) candidate: Option<String>,

    #[arg(long)]
    pub(crate) candidate_argv: Option<String>,

    #[arg(long)]
    pub(crate) secondary: Option<String>,

    #[arg(long)]
    pub(crate) secondary_argv: Option<String>,

    #[arg(long, default_value_t = DEFAULT_MAX_BODY_CAPTURE_BYTES)]
    pub(crate) max_body_capture_bytes: usize,

    #[arg(long = "ignored-json-path")]
    pub(crate) ignored_json_paths: Vec<String>,

    #[arg(long = "ignored-header")]
    pub(crate) ignored_headers: Vec<String>,

    #[arg(long)]
    pub(crate) ignore_stderr: bool,

    #[arg(long)]
    pub(crate) serial_targets: bool,

    #[arg(long)]
    pub(crate) quiet: bool,

    #[arg(long)]
    pub(crate) compact: bool,
}

#[derive(Debug, Args)]
pub(crate) struct BatchArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(long, default_value = "-")]
    pub(crate) input: PathBuf,

    #[arg(long)]
    pub(crate) jobs: Option<usize>,

    #[arg(long)]
    pub(crate) quiet: bool,

    #[arg(long)]
    pub(crate) emit_runs: bool,

    #[arg(long)]
    pub(crate) serial_targets: bool,
}
