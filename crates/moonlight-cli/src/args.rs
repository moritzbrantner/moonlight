use crate::config::DEFAULT_MAX_BODY_CAPTURE_BYTES;
use clap::{Args, Parser, Subcommand};
use std::{env, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(about = "Compare command output with Moonlight's shared comparison engine")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CliCommand {
    #[command(
        about = "Run one primary/candidate comparison",
        after_help = "Examples:\n  moonlight run --primary 'printf \"{\\\"value\\\":42}\\n\"' --candidate 'printf \"{\\\"value\\\":43}\\n\"'\n  moonlight run --primary-argv '[\"printf\",\"%s\\n\",\"{\\\"value\\\":42}\"]' --candidate-argv '[\"printf\",\"%s\\n\",\"{\\\"value\\\":42}\"]' --compact"
    )]
    Run(RunArgs),
    #[command(
        about = "Run many JSONL comparison cases in one process",
        after_help = "Examples:\n  moonlight batch --input cases.jsonl --jobs 8\n  moonlight batch --input cases.jsonl --emit-runs"
    )]
    Batch(BatchArgs),
    #[command(
        alias = "ls",
        about = "List stored comparison runs newest first",
        after_help = "Examples:\n  moonlight list --limit 20\n  moonlight ls --limit 20 --offset 20 --compact"
    )]
    List(ListArgs),
    #[command(
        about = "Summarize stored comparison runs",
        after_help = "Examples:\n  moonlight stats\n  moonlight stats --compact"
    )]
    Stats(ReadArgs),
    #[command(
        about = "Show a full stored comparison run",
        after_help = "Examples:\n  moonlight show 00000000-0000-0000-0000-000000000000\n  moonlight show 00000000-0000-0000-0000-000000000000 --compact"
    )]
    Show(ShowArgs),
}

#[derive(Debug, Args)]
pub(crate) struct StorageArgs {
    #[arg(
        long,
        value_name = "PATH",
        help = "JSONL storage file. Defaults to MOONLIGHT_CLI_STORAGE_PATH or data/moonlight/cli-runs.jsonl"
    )]
    storage_path: Option<PathBuf>,
}

impl StorageArgs {
    pub(crate) fn storage_path(&self) -> PathBuf {
        self.storage_path
            .clone()
            .unwrap_or_else(default_storage_path)
    }
}

#[derive(Debug, Args)]
pub(crate) struct ReadArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(long, help = "Print compact JSON instead of pretty JSON")]
    pub(crate) compact: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ListArgs {
    #[command(flatten)]
    pub(crate) output: ReadArgs,

    #[arg(long, help = "Maximum number of newest runs to print")]
    pub(crate) limit: Option<usize>,

    #[arg(long, default_value_t = 0, help = "Number of newest runs to skip")]
    pub(crate) offset: usize,
}

#[derive(Debug, Args)]
pub(crate) struct ShowArgs {
    pub(crate) id: Uuid,

    #[command(flatten)]
    pub(crate) output: ReadArgs,
}

#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(long, help = "Primary target shell command run through sh -lc")]
    pub(crate) primary: Option<String>,

    #[arg(
        long,
        help = "Primary target argv as a JSON string array, bypassing sh -lc"
    )]
    pub(crate) primary_argv: Option<String>,

    #[arg(long, help = "Candidate target shell command run through sh -lc")]
    pub(crate) candidate: Option<String>,

    #[arg(
        long,
        help = "Candidate target argv as a JSON string array, bypassing sh -lc"
    )]
    pub(crate) candidate_argv: Option<String>,

    #[arg(
        long,
        help = "Optional secondary reference shell command run through sh -lc"
    )]
    pub(crate) secondary: Option<String>,

    #[arg(
        long,
        help = "Optional secondary reference argv as a JSON string array"
    )]
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

    #[arg(long, help = "Print no run JSON to stdout")]
    pub(crate) quiet: bool,

    #[arg(long, help = "Print one-line JSON instead of pretty JSON")]
    pub(crate) compact: bool,
}

#[derive(Debug, Args)]
pub(crate) struct BatchArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(
        long,
        default_value = "-",
        help = "JSONL batch input path, or - for stdin"
    )]
    pub(crate) input: PathBuf,

    #[arg(long, help = "Maximum number of cases to run concurrently")]
    pub(crate) jobs: Option<usize>,

    #[arg(long, help = "Suppress the final batch summary")]
    pub(crate) quiet: bool,

    #[arg(long, help = "Print compact run JSONL records as cases complete")]
    pub(crate) emit_runs: bool,

    #[arg(
        long,
        help = "Run primary, candidate, and secondary targets sequentially per case"
    )]
    pub(crate) serial_targets: bool,
}

fn default_storage_path() -> PathBuf {
    env::var_os("MOONLIGHT_CLI_STORAGE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/moonlight/cli-runs.jsonl"))
}
