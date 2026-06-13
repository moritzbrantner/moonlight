use anyhow::{bail, Context};
use bytes::Bytes;
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use futures::{stream, StreamExt};
use moonlight_core::{
    compare::{capture_body, compare_targets, CapturedTarget, CompareConfig},
    storage::{RunWriter, Storage},
    Adapter, Classification, ComparisonRun, RunInput, TargetObservation,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::Instant};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, BufReader},
    process::Command,
};
use uuid::Uuid;

const DEFAULT_IGNORED_JSON_PATHS: &[&str] = &["$.timestamp", "$.requestId", "$.traceId", "$.id"];
const DEFAULT_IGNORED_HEADERS: &[&str] = &[
    "date",
    "server",
    "set-cookie",
    "x-request-id",
    "traceparent",
];
const DEFAULT_MAX_BODY_CAPTURE_BYTES: usize = 8192;

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
    Batch(BatchArgs),
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

    #[arg(long, default_value_t = DEFAULT_MAX_BODY_CAPTURE_BYTES)]
    max_body_capture_bytes: usize,

    #[arg(long = "ignored-json-path")]
    ignored_json_paths: Vec<String>,

    #[arg(long = "ignored-header")]
    ignored_headers: Vec<String>,

    #[arg(long)]
    ignore_stderr: bool,

    #[arg(long)]
    serial_targets: bool,

    #[arg(long)]
    quiet: bool,
}

#[derive(Debug, Args)]
struct BatchArgs {
    #[command(flatten)]
    storage: StorageArgs,

    #[arg(long, default_value = "-")]
    input: PathBuf,

    #[arg(long)]
    jobs: Option<usize>,

    #[arg(long)]
    quiet: bool,

    #[arg(long)]
    emit_runs: bool,

    #[arg(long)]
    serial_targets: bool,
}

#[derive(Debug, Clone)]
struct Case {
    primary: String,
    candidate: String,
    secondary: Option<String>,
    max_body_capture_bytes: usize,
    ignored_json_paths: Vec<String>,
    ignored_headers: Vec<String>,
    ignore_stderr: bool,
}

#[derive(Debug, Deserialize)]
struct BatchCase {
    primary: String,
    candidate: String,
    #[serde(default)]
    secondary: Option<String>,
    #[serde(default)]
    max_body_capture_bytes: Option<usize>,
    #[serde(default)]
    ignored_json_paths: Vec<String>,
    #[serde(default)]
    ignored_headers: Vec<String>,
    #[serde(default)]
    ignore_stderr: bool,
}

#[derive(Debug)]
struct CapturedTargets {
    primary: CapturedTarget,
    candidate: CapturedTarget,
    secondary: Option<CapturedTarget>,
}

#[derive(Debug, Default, Serialize)]
struct BatchSummary {
    total_runs: usize,
    matches: usize,
    suspicious_differences: usize,
    reference_noise: usize,
    suspicious_with_noise: usize,
    target_errors: usize,
    duration_ms: u128,
    jobs: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommand::Run(args) => run(args).await,
        CliCommand::Batch(args) => batch(args).await,
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
    let case = Case {
        primary: args.primary,
        candidate: args.candidate,
        secondary: args.secondary,
        max_body_capture_bytes: args.max_body_capture_bytes,
        ignored_json_paths: args.ignored_json_paths,
        ignored_headers: args.ignored_headers,
        ignore_stderr: args.ignore_stderr,
    };
    let run = execute_case(case, args.serial_targets).await;

    let writer = RunWriter::open(args.storage.storage_path).await?;
    writer.append(&run).await?;
    writer.flush().await?;

    if !args.quiet {
        println!("{}", serde_json::to_string_pretty(&run)?);
    }
    Ok(())
}

async fn batch(args: BatchArgs) -> anyhow::Result<()> {
    if args.quiet && args.emit_runs {
        bail!("--quiet and --emit-runs cannot be used together");
    }

    let jobs = args.jobs.unwrap_or_else(default_jobs).max(1);
    let cases = read_batch_cases(&args.input).await?;
    let writer = RunWriter::open(args.storage.storage_path).await?;
    let writer = Arc::new(writer);
    let started = Instant::now();
    let mut summary = BatchSummary {
        jobs,
        ..BatchSummary::default()
    };

    let mut runs = stream::iter(cases.into_iter().map(|case| {
        let serial_targets = args.serial_targets;
        async move { execute_case(case, serial_targets).await }
    }))
    .buffer_unordered(jobs);

    while let Some(run) = runs.next().await {
        writer.append(&run).await?;
        summary.record(&run.comparison.classification);
        if args.emit_runs {
            println!("{}", serde_json::to_string(&run)?);
        }
    }
    writer.flush().await?;
    summary.duration_ms = started.elapsed().as_millis();

    if !args.quiet && !args.emit_runs {
        println!("{}", serde_json::to_string(&summary)?);
    }
    Ok(())
}

async fn read_batch_cases(input: &PathBuf) -> anyhow::Result<Vec<Case>> {
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
        if case.primary.trim().is_empty() {
            bail!("invalid batch JSONL on line {line_number}: primary must not be empty");
        }
        if case.candidate.trim().is_empty() {
            bail!("invalid batch JSONL on line {line_number}: candidate must not be empty");
        }
        cases.push(Case {
            primary: case.primary,
            candidate: case.candidate,
            secondary: case.secondary,
            max_body_capture_bytes: case
                .max_body_capture_bytes
                .unwrap_or(DEFAULT_MAX_BODY_CAPTURE_BYTES),
            ignored_json_paths: case.ignored_json_paths,
            ignored_headers: case.ignored_headers,
            ignore_stderr: case.ignore_stderr,
        });
    }

    Ok(cases)
}

async fn read_stdin_lines() -> anyhow::Result<Vec<String>> {
    let mut lines = BufReader::new(io::stdin()).lines();
    let mut output = Vec::new();
    while let Some(line) = lines.next_line().await? {
        output.push(line);
    }
    Ok(output)
}

async fn execute_case(case: Case, serial_targets: bool) -> ComparisonRun {
    let targets = run_targets(&case, serial_targets).await;
    let compare_config = build_compare_config(
        &case.ignored_json_paths,
        &case.ignored_headers,
        case.ignore_stderr,
    );
    let comparison = compare_targets(
        &targets.primary,
        &targets.candidate,
        targets.secondary.as_ref(),
        &compare_config,
    );

    ComparisonRun {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        adapter: Adapter::Cli,
        input: RunInput::Cli {
            primary_command: case.primary,
            candidate_command: case.candidate,
            secondary_command: case.secondary,
        },
        request_headers: BTreeMap::new(),
        request_body: capture_body(&[], case.max_body_capture_bytes),
        primary: targets.primary.observation,
        candidate: targets.candidate.observation,
        secondary: targets.secondary.map(|target| target.observation),
        comparison,
    }
}

async fn run_targets(case: &Case, serial_targets: bool) -> CapturedTargets {
    if serial_targets {
        let primary = run_command("primary", &case.primary, case.max_body_capture_bytes).await;
        let candidate =
            run_command("candidate", &case.candidate, case.max_body_capture_bytes).await;
        let secondary = match &case.secondary {
            Some(command) => {
                Some(run_command("secondary", command, case.max_body_capture_bytes).await)
            }
            None => None,
        };
        return CapturedTargets {
            primary,
            candidate,
            secondary,
        };
    }

    match &case.secondary {
        Some(secondary_command) => {
            let (primary, candidate, secondary) = tokio::join!(
                run_command("primary", &case.primary, case.max_body_capture_bytes),
                run_command("candidate", &case.candidate, case.max_body_capture_bytes),
                run_command("secondary", secondary_command, case.max_body_capture_bytes),
            );
            CapturedTargets {
                primary,
                candidate,
                secondary: Some(secondary),
            }
        }
        None => {
            let (primary, candidate) = tokio::join!(
                run_command("primary", &case.primary, case.max_body_capture_bytes),
                run_command("candidate", &case.candidate, case.max_body_capture_bytes),
            );
            CapturedTargets {
                primary,
                candidate,
                secondary: None,
            }
        }
    }
}

fn build_compare_config(
    ignored_json_paths: &[String],
    ignored_headers: &[String],
    ignore_stderr: bool,
) -> CompareConfig {
    let ignored_json_paths = values_or_defaults(ignored_json_paths, DEFAULT_IGNORED_JSON_PATHS);
    let ignored_headers = values_or_defaults(ignored_headers, DEFAULT_IGNORED_HEADERS);
    CompareConfig::new(&ignored_json_paths, &ignored_headers, ignore_stderr)
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

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

impl BatchSummary {
    fn record(&mut self, classification: &Classification) {
        self.total_runs += 1;
        match classification {
            Classification::Match => self.matches += 1,
            Classification::SuspiciousDifference => self.suspicious_differences += 1,
            Classification::ReferenceNoise => self.reference_noise += 1,
            Classification::SuspiciousWithNoise => self.suspicious_with_noise += 1,
            Classification::TargetError => self.target_errors += 1,
        }
    }
}
