mod args;
mod batch;
mod command;
mod command_form;
mod config;
mod execute;
mod input;
mod run_once;
mod types;

use anyhow::Context;
use args::{Cli, CliCommand};
use clap::Parser;
use config::CliDefaults;
use moonlight_core::{
    config::load_optional_config,
    report::{render_report, ReportFormat},
    review::{ReviewStatus, ReviewStore, ReviewUpdate},
    storage::JsonlStorageReader,
    Adapter, Classification, RunFilter,
};

pub async fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let file_config = load_optional_config(cli.config.config.as_deref(), cli.config.no_config)?;
    let defaults = CliDefaults::from_config(&file_config);

    match cli.command {
        CliCommand::Run(args) => run_once::run(args, &defaults).await,
        CliCommand::Batch(args) => batch::batch(args, &defaults).await,
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
            Ok(())
        }
        CliCommand::Stats(args) => {
            let storage = JsonlStorageReader::new(storage_path(args.storage, &defaults));
            print_json(&storage.stats().await?, args.compact)?;
            Ok(())
        }
        CliCommand::Show(args) => {
            let storage = JsonlStorageReader::new(storage_path(args.output.storage, &defaults));
            let run = storage
                .get(args.id)
                .await?
                .with_context(|| format!("comparison run {} was not found", args.id))?;
            print_json(&run, args.output.compact)?;
            Ok(())
        }
        CliCommand::Report(args) => {
            let storage = JsonlStorageReader::new(storage_path(args.storage, &defaults));
            let run = storage
                .get(args.id)
                .await?
                .with_context(|| format!("comparison run {} was not found", args.id))?;
            let format = args.format.parse::<ReportFormat>()?;
            println!("{}", render_report(&run, None, format)?);
            Ok(())
        }
        CliCommand::Review(args) => {
            let store = ReviewStore::load(defaults.review_state_path.clone()).await?;
            let state = store
                .put(
                    args.id,
                    ReviewUpdate {
                        status: args.status.parse::<ReviewStatus>()?,
                        note: args.note,
                        tags: args.tags,
                    },
                )
                .await?;
            print_json(&state, false)?;
            Ok(())
        }
        CliCommand::Reviews(args) => {
            let store = ReviewStore::load(defaults.review_state_path.clone()).await?;
            let status = args
                .status
                .map(|status| status.parse::<ReviewStatus>())
                .transpose()?;
            print_json(&store.list(status).await, args.compact)?;
            Ok(())
        }
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
