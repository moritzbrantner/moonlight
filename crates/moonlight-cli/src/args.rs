use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(
    name = "moonlight",
    bin_name = "moonlight",
    version,
    about = "Compare command behavior with Moonlight's shared comparison engine"
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) config: ConfigArgs,

    #[command(subcommand)]
    pub(crate) command: CliCommand,
}

#[derive(Debug, Args)]
pub(crate) struct ConfigArgs {
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help_heading = "Config",
        conflicts_with = "no_config",
        help = "TOML config file to read instead of ./moonlight.conf"
    )]
    pub(crate) config: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        help_heading = "Config",
        help = "Do not read ./moonlight.conf"
    )]
    pub(crate) no_config: bool,
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
    #[command(
        about = "Render a shareable report for a stored comparison run",
        after_help = "Examples:\n  moonlight report 00000000-0000-0000-0000-000000000000\n  moonlight report 00000000-0000-0000-0000-000000000000 --format json"
    )]
    Report(ReportArgs),
    #[command(
        about = "Set review state for a stored comparison run",
        after_help = "Examples:\n  moonlight review 00000000-0000-0000-0000-000000000000 --status ignored --note 'Known rollout delta'"
    )]
    Review(ReviewArgs),
    #[command(
        about = "List stored run review states",
        after_help = "Examples:\n  moonlight reviews\n  moonlight reviews --status new"
    )]
    Reviews(ReviewsArgs),
    #[command(about = "Evaluate coding-agent patches against existing projects")]
    #[command(subcommand)]
    Eval(EvalCommand),
}

#[derive(Debug, Subcommand)]
pub(crate) enum EvalCommand {
    #[command(
        about = "Run project checks against baseline and candidate worktrees",
        after_help = "Examples:\n  moonlight eval run --project moonlight.eval.toml --candidate-patch agent.patch\n  moonlight eval run --project moonlight.eval.toml --candidate-ref agent/output-branch"
    )]
    Run(EvalRunArgs),
    #[command(
        about = "Render a stored project eval summary",
        after_help = "Examples:\n  moonlight eval report 00000000-0000-0000-0000-000000000000 --format markdown"
    )]
    Report(EvalReportArgs),
}

#[derive(Debug, Args)]
pub(crate) struct StorageArgs {
    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Storage",
        help = "JSONL storage file"
    )]
    pub(crate) storage_path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct EvalRunArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(
        long,
        value_name = "PATH",
        default_value = "moonlight.eval.toml",
        help_heading = "Project",
        help = "Project eval TOML config"
    )]
    pub(crate) project: PathBuf,

    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Project",
        help = "Repository path override"
    )]
    pub(crate) repo: Option<PathBuf>,

    #[arg(
        long,
        value_name = "REF",
        help_heading = "Project",
        help = "Baseline ref override"
    )]
    pub(crate) baseline_ref: Option<String>,

    #[arg(
        long,
        value_name = "REF",
        conflicts_with = "candidate_patch",
        help_heading = "Candidate",
        help = "Candidate branch or commit"
    )]
    pub(crate) candidate_ref: Option<String>,

    #[arg(
        long,
        value_name = "PATH",
        conflicts_with = "candidate_ref",
        help_heading = "Candidate",
        help = "Patch to apply on top of the baseline ref"
    )]
    pub(crate) candidate_patch: Option<PathBuf>,

    #[arg(
        long,
        value_name = "FORMAT",
        default_value = "text",
        value_parser = ["text", "json", "markdown"],
        help_heading = "Output",
        help = "Summary output format"
    )]
    pub(crate) format: String,

    #[arg(
        long,
        value_name = "MODE",
        value_parser = ["never", "failed", "always"],
        help_heading = "Execution",
        help = "Override when eval worktrees are kept"
    )]
    pub(crate) keep_worktrees: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help_heading = "Execution",
        help = "Maximum number of checks to run concurrently"
    )]
    pub(crate) jobs: Option<usize>,

    #[arg(long, help_heading = "Output", help = "Suppress summary output")]
    pub(crate) quiet: bool,
}

#[derive(Debug, Args)]
pub(crate) struct EvalReportArgs {
    pub(crate) eval_id: Uuid,

    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(
        long,
        value_name = "FORMAT",
        default_value = "markdown",
        value_parser = ["text", "json", "markdown"],
        help_heading = "Output",
        help = "Summary output format"
    )]
    pub(crate) format: String,
}

#[derive(Debug, Args)]
pub(crate) struct ReadArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(long, help_heading = "Output", help = "Print compact JSON")]
    pub(crate) compact: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ListArgs {
    #[command(flatten)]
    pub(crate) output: ReadArgs,

    #[arg(
        long,
        help_heading = "Output",
        help = "Maximum number of newest runs to print"
    )]
    pub(crate) limit: Option<usize>,

    #[arg(
        long,
        default_value_t = 0,
        help_heading = "Output",
        help = "Number of newest runs to skip"
    )]
    pub(crate) offset: usize,

    #[arg(
        long,
        value_name = "CLASSIFICATION",
        value_parser = ["match", "suspicious_difference", "reference_noise", "suspicious_with_noise", "target_error"],
        help_heading = "Filters",
        help = "Only list runs with this classification"
    )]
    pub(crate) classification: Option<String>,

    #[arg(
        long,
        value_name = "ADAPTER",
        value_parser = ["http", "cli", "project"],
        help_heading = "Filters",
        help = "Only list runs from this adapter"
    )]
    pub(crate) adapter: Option<String>,

    #[arg(
        long = "query",
        alias = "q",
        value_name = "TEXT",
        help_heading = "Filters",
        help = "Search run input text"
    )]
    pub(crate) query: Option<String>,

    #[arg(
        long,
        value_name = "STATUS",
        help_heading = "Filters",
        help = "Only list runs where any target has this status"
    )]
    pub(crate) status: Option<u16>,
}

#[derive(Debug, Args)]
pub(crate) struct ShowArgs {
    pub(crate) id: Uuid,

    #[command(flatten)]
    pub(crate) output: ReadArgs,
}

#[derive(Debug, Args)]
pub(crate) struct ReportArgs {
    pub(crate) id: Uuid,

    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(
        long,
        value_name = "FORMAT",
        default_value = "markdown",
        value_parser = ["markdown", "json"],
        help_heading = "Output",
        help = "Report output format"
    )]
    pub(crate) format: String,
}

#[derive(Debug, Args)]
pub(crate) struct ReviewArgs {
    pub(crate) id: Uuid,

    #[arg(
        long,
        value_name = "STATUS",
        value_parser = ["new", "accepted", "ignored", "fixed"],
        help = "Review status to store"
    )]
    pub(crate) status: String,

    #[arg(long, value_name = "TEXT", help = "Optional review note")]
    pub(crate) note: Option<String>,

    #[arg(
        long = "tag",
        value_name = "TAG",
        help = "Review tag; repeat to add multiple tags"
    )]
    pub(crate) tags: Vec<String>,
}

#[derive(Debug, Args)]
pub(crate) struct ReviewsArgs {
    #[arg(
        long,
        value_name = "STATUS",
        value_parser = ["new", "accepted", "ignored", "fixed"],
        help = "Only list reviews with this status"
    )]
    pub(crate) status: Option<String>,

    #[arg(long, help_heading = "Output", help = "Print compact JSON")]
    pub(crate) compact: bool,
}

#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(
        long,
        help_heading = "Target Commands",
        help = "Primary reference shell command run through sh -lc"
    )]
    pub(crate) primary: Option<String>,

    #[arg(
        long,
        value_name = "JSON",
        help_heading = "Target Commands",
        help = "Primary reference argv as a JSON string array, bypassing sh -lc"
    )]
    pub(crate) primary_argv: Option<String>,

    #[arg(
        long,
        help_heading = "Target Commands",
        help = "Candidate shell command run through sh -lc"
    )]
    pub(crate) candidate: Option<String>,

    #[arg(
        long,
        value_name = "JSON",
        help_heading = "Target Commands",
        help = "Candidate argv as a JSON string array, bypassing sh -lc"
    )]
    pub(crate) candidate_argv: Option<String>,

    #[arg(
        long,
        help_heading = "Target Commands",
        help = "Optional secondary reference shell command run through sh -lc"
    )]
    pub(crate) secondary: Option<String>,

    #[arg(
        long,
        value_name = "JSON",
        help_heading = "Target Commands",
        help = "Optional secondary reference argv as a JSON string array"
    )]
    pub(crate) secondary_argv: Option<String>,

    #[arg(
        long,
        value_name = "BYTES",
        help_heading = "Comparison",
        help = "Maximum stdout/stderr body bytes to store"
    )]
    pub(crate) max_body_capture_bytes: Option<usize>,

    #[arg(
        long = "ignore-json-path",
        value_name = "PATH",
        help_heading = "Comparison",
        help = "Exact JSON diff path to ignore"
    )]
    pub(crate) ignore_json_paths: Vec<String>,

    #[arg(
        long = "ignore-json-path-pattern",
        value_name = "PATTERN",
        help_heading = "Comparison",
        help = "JSON diff path pattern to ignore; supports * and [*]"
    )]
    pub(crate) ignore_json_path_patterns: Vec<String>,

    #[arg(
        long = "redact-json-path",
        value_name = "PATH",
        help_heading = "Redaction",
        help = "Exact JSON body path to redact from stored previews and diffs"
    )]
    pub(crate) redact_json_paths: Vec<String>,

    #[arg(
        long = "redact-json-path-pattern",
        value_name = "PATTERN",
        help_heading = "Redaction",
        help = "JSON body path pattern to redact from stored previews and diffs; supports * and [*]"
    )]
    pub(crate) redact_json_path_patterns: Vec<String>,

    #[arg(
        long = "ignore-header",
        value_name = "HEADER",
        help_heading = "Comparison",
        help = "Header name to ignore during comparison"
    )]
    pub(crate) ignore_headers: Vec<String>,

    #[arg(long, help_heading = "Comparison", help = "Ignore stderr differences")]
    pub(crate) ignore_stderr: bool,

    #[arg(
        long,
        value_name = "MS",
        help_heading = "Execution",
        help = "Per-target timeout in milliseconds; 0 uses the default"
    )]
    pub(crate) target_timeout_ms: Option<u64>,

    #[arg(
        long,
        help_heading = "Execution",
        help = "Run primary, candidate, and secondary targets sequentially"
    )]
    pub(crate) serial_targets: bool,

    #[arg(long, help_heading = "Output", help = "Print no run JSON to stdout")]
    pub(crate) quiet: bool,

    #[arg(
        long,
        help_heading = "Output",
        help = "Print one-line JSON instead of pretty JSON"
    )]
    pub(crate) compact: bool,
}

#[derive(Debug, Args)]
pub(crate) struct BatchArgs {
    #[command(flatten)]
    pub(crate) storage: StorageArgs,

    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Input",
        help = "JSONL batch input path, or - for stdin"
    )]
    pub(crate) input: Option<PathBuf>,

    #[arg(
        long,
        value_name = "N",
        help_heading = "Execution",
        help = "Maximum number of cases to run concurrently"
    )]
    pub(crate) jobs: Option<usize>,

    #[arg(
        long,
        help_heading = "Output",
        help = "Suppress the final batch summary"
    )]
    pub(crate) quiet: bool,

    #[arg(
        long,
        help_heading = "Output",
        help = "Print compact run JSONL records as cases complete"
    )]
    pub(crate) emit_runs: bool,

    #[arg(
        long,
        help_heading = "Execution",
        help = "Run primary, candidate, and secondary targets sequentially per case"
    )]
    pub(crate) serial_targets: bool,
}
