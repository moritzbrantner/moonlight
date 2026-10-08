mod args;
mod batch;
mod command;
mod command_form;
mod config;
mod eval;
mod eval_config;
mod execute;
mod input;
#[cfg(target_os = "linux")]
mod linux_process;
mod run_once;
mod types;
#[cfg(windows)]
mod windows_job;
mod worktree;

use anyhow::Context;
use args::{Cli, CliCommand, EvalCommand};
use clap::Parser;
use config::CliDefaults;
use moonlight_core::{
    config::load_optional_config,
    report::{render_report, ReportFormat},
    review::{ReviewStatus, ReviewStore, ReviewUpdate},
    storage::JsonlStorageReader,
    Adapter, Classification, RunFilter,
};
use std::process::ExitCode;

#[derive(Debug)]
pub struct ExitCodeError {
    code: u8,
    message: String,
}

impl ExitCodeError {
    fn new(code: u8, error: anyhow::Error) -> Self {
        Self {
            code,
            message: format!("{error:#}"),
        }
    }

    pub fn code(&self) -> u8 {
        self.code
    }
}

impl std::fmt::Display for ExitCodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for ExitCodeError {}

#[doc(hidden)]
pub fn main_entry() -> ExitCode {
    #[cfg(target_os = "linux")]
    if linux_process::is_supervisor_request() {
        return if linux_process::run_supervisor().is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(anyhow::Error::from)
        .and_then(|runtime| runtime.block_on(run_cli()));
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error:#}");
            error
                .downcast_ref::<ExitCodeError>()
                .map(|error| ExitCode::from(error.code()))
                .unwrap_or(ExitCode::from(1))
        }
    }
}

pub async fn run_cli() -> anyhow::Result<ExitCode> {
    let result = tokio::select! {
        biased;
        signal = shutdown_signal() => Ok(ExitCode::from(signal?)),
        result = run_cli_inner() => result,
    };
    #[cfg(target_os = "linux")]
    linux_process::reap_dropped_supervisors().await?;
    result
}

async fn shutdown_signal() -> std::io::Result<u8> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut interrupt = signal(SignalKind::interrupt())?;
        let mut terminate = signal(SignalKind::terminate())?;
        let mut hangup = signal(SignalKind::hangup())?;
        tokio::select! {
            _ = interrupt.recv() => Ok(130),
            _ = terminate.recv() => Ok(143),
            _ = hangup.recv() => Ok(129),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        Ok(130)
    }
}

async fn run_cli_inner() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let file_config = load_optional_config(cli.config.config.as_deref(), cli.config.no_config)?;
    let defaults = CliDefaults::from_config(&file_config);

    match cli.command {
        CliCommand::Run(args) => {
            run_once::run(args, &defaults).await?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Batch(args) => {
            batch::batch(args, &defaults).await?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::List(args) => {
            let storage = JsonlStorageReader::new(storage_path(args.output.storage, &defaults));
            let page = storage
                .filtered_page(
                    &run_filter(args.classification, args.adapter, args.query, args.status)?,
                    args.limit.unwrap_or(100),
                    args.offset,
                )
                .await?;
            print_json(&page, args.output.compact)?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Stats(args) => {
            let storage = JsonlStorageReader::new(storage_path(args.storage, &defaults));
            print_json(&storage.stats().await?, args.compact)?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Show(args) => {
            let storage = JsonlStorageReader::new(storage_path(args.output.storage, &defaults));
            let run = storage
                .get(args.id)
                .await?
                .with_context(|| format!("comparison run {} was not found", args.id))?;
            print_json(&run, args.output.compact)?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Report(args) => {
            let storage = JsonlStorageReader::new(storage_path(args.storage, &defaults));
            let run = storage
                .get(args.id)
                .await?
                .with_context(|| format!("comparison run {} was not found", args.id))?;
            let format = args.format.parse::<ReportFormat>()?;
            println!("{}", render_report(&run, None, format)?);
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Review(args) => {
            let store = ReviewStore::load(defaults.review_state_path.clone()).await?;
            let state = store
                .put(
                    args.id,
                    ReviewUpdate {
                        status: args.status.parse::<ReviewStatus>()?,
                        note: args.note,
                        tags: Some(args.tags),
                    },
                )
                .await?;
            print_json(&state, false)?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Reviews(args) => {
            let store = ReviewStore::load(defaults.review_state_path.clone()).await?;
            let status = args
                .status
                .map(|status| status.parse::<ReviewStatus>())
                .transpose()?;
            print_json(&store.list(status).await, args.compact)?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Eval(command) => match command {
            EvalCommand::Run(args) => eval::run(args, &defaults)
                .await
                .map_err(|error| ExitCodeError::new(2, error).into()),
            EvalCommand::Report(args) => eval::report(args, &defaults)
                .await
                .map_err(|error| ExitCodeError::new(2, error).into()),
        },
    }
}

fn storage_path(args: args::StorageArgs, defaults: &CliDefaults) -> std::path::PathBuf {
    args.storage_path
        .unwrap_or_else(|| defaults.storage_path.clone())
}

fn print_json(value: &impl serde::Serialize, compact: bool) -> anyhow::Result<()> {
    if compact {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

fn run_filter(
    classification: Option<String>,
    adapter: Option<String>,
    query: Option<String>,
    status: Option<u16>,
) -> anyhow::Result<RunFilter> {
    Ok(RunFilter {
        classification: classification
            .map(|value| value.parse::<Classification>())
            .transpose()?,
        adapter: adapter.map(|value| value.parse::<Adapter>()).transpose()?,
        query,
        status,
        has_noise: None,
        has_diff: None,
    })
}
