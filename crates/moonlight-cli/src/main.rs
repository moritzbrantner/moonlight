use anyhow::Context;
use bytes::Bytes;
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use moonlight_core::{
    compare::{capture_body, compare_targets, CapturedTarget, CompareConfig},
    storage::Storage,
    Adapter, ComparisonRun, RunInput, TargetObservation,
};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};
use tokio::process::Command;
use uuid::Uuid;

const DEFAULT_IGNORED_JSON_PATHS: &[&str] = &["$.timestamp", "$.requestId", "$.traceId", "$.id"];
const DEFAULT_IGNORED_HEADERS: &[&str] = &[
    "date",
    "server",
    "set-cookie",
    "x-request-id",
    "traceparent",
];

#[derive(Debug, Parser)]
#[command(name = "moonlight-cli")]
#[command(about = "Compare command output with Moonlight's shared comparison engine")]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    Run(RunArgs),
    List(StorageArgs),
    Stats(StorageArgs),
    Show(ShowArgs),
}

#[derive(Debug, Args)]
struct StorageArgs {
    #[arg(long, default_value = "data/moonlight/cli-runs.jsonl")]
    storage_path: PathBuf,
}

#[derive(Debug, Args)]
struct ShowArgs {
    id: Uuid,

    #[command(flatten)]
    storage: StorageArgs,
}

#[derive(Debug, Args)]
struct RunArgs {
    #[command(flatten)]
    storage: StorageArgs,

    #[arg(long)]
    primary: String,

    #[arg(long)]
    candidate: String,

    #[arg(long)]
    secondary: Option<String>,

    #[arg(long, default_value_t = 8192)]
    max_body_capture_bytes: usize,

    #[arg(long = "ignored-json-path")]
    ignored_json_paths: Vec<String>,

    #[arg(long = "ignored-header")]
    ignored_headers: Vec<String>,

    #[arg(long)]
    ignore_stderr: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommand::Run(args) => run(args).await,
        CliCommand::List(args) => {
            let storage = Storage::load(args.storage_path).await?;
            println!("{}", serde_json::to_string_pretty(&storage.list().await)?);
            Ok(())
        }
        CliCommand::Stats(args) => {
            let storage = Storage::load(args.storage_path).await?;
            println!("{}", serde_json::to_string_pretty(&storage.stats().await)?);
            Ok(())
        }
        CliCommand::Show(args) => {
            let storage = Storage::load(args.storage.storage_path).await?;
            let run = storage
                .get(args.id)
                .await
                .with_context(|| format!("comparison run {} was not found", args.id))?;
            println!("{}", serde_json::to_string_pretty(&run)?);
            Ok(())
        }
    }
}

async fn run(args: RunArgs) -> anyhow::Result<()> {
    let primary = run_command("primary", &args.primary, args.max_body_capture_bytes).await;
    let candidate = run_command("candidate", &args.candidate, args.max_body_capture_bytes).await;
    let secondary = match &args.secondary {
        Some(command) => Some(run_command("secondary", command, args.max_body_capture_bytes).await),
        None => None,
    };
    let ignored_json_paths =
        values_or_defaults(&args.ignored_json_paths, DEFAULT_IGNORED_JSON_PATHS);
    let ignored_headers = values_or_defaults(&args.ignored_headers, DEFAULT_IGNORED_HEADERS);
    let compare_config =
        CompareConfig::new(&ignored_json_paths, &ignored_headers, args.ignore_stderr);
    let comparison = compare_targets(&primary, &candidate, secondary.as_ref(), &compare_config);
    let id = Uuid::new_v4();
    let run = ComparisonRun {
        id,
        timestamp: Utc::now(),
        adapter: Adapter::Cli,
        input: RunInput::Cli {
            primary_command: args.primary.clone(),
            candidate_command: args.candidate.clone(),
            secondary_command: args.secondary.clone(),
        },
        request_headers: BTreeMap::new(),
        request_body: capture_body(&[], args.max_body_capture_bytes),
        primary: primary.observation,
        candidate: candidate.observation,
        secondary: secondary.map(|target| target.observation),
        comparison,
    };

    let storage = Storage::load(args.storage.storage_path).await?;
    storage.insert(run.clone()).await?;
    println!("{}", serde_json::to_string_pretty(&run)?);
    Ok(())
}

async fn run_command(
    label: &'static str,
    command: &str,
    max_body_capture_bytes: usize,
) -> CapturedTarget {
    let started = Instant::now();
    match Command::new("sh").arg("-lc").arg(command).output().await {
        Ok(output) => {
            let stderr = capture_body(&output.stderr, max_body_capture_bytes);
            let error = output
                .status
                .code()
                .is_none()
                .then(|| format!("{label} command terminated by signal"));

            CapturedTarget {
                observation: TargetObservation {
                    status: output
                        .status
                        .code()
                        .and_then(|code| u16::try_from(code).ok()),
                    headers: BTreeMap::new(),
                    body: capture_body(&output.stdout, max_body_capture_bytes),
                    stderr: Some(stderr),
                    latency_ms: started.elapsed().as_millis(),
                    error,
                },
                body_bytes: Bytes::from(output.stdout),
                stderr_bytes: Bytes::from(output.stderr),
            }
        }
        Err(error) => CapturedTarget {
            observation: TargetObservation {
                status: None,
                headers: BTreeMap::new(),
                body: capture_body(&[], max_body_capture_bytes),
                stderr: Some(capture_body(&[], max_body_capture_bytes)),
                latency_ms: started.elapsed().as_millis(),
                error: Some(format!("{label} command failed to start: {error}")),
            },
            body_bytes: Bytes::new(),
            stderr_bytes: Bytes::new(),
        },
    }
}

fn values_or_defaults(values: &[String], defaults: &[&str]) -> Vec<String> {
    if values.is_empty() {
        defaults.iter().map(|value| (*value).to_string()).collect()
    } else {
        values.to_vec()
    }
}
