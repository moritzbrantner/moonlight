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
use moonlight_core::{config::load_optional_config, storage::JsonlStorageReader};

pub async fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let file_config = load_optional_config(cli.config.config.as_deref(), cli.config.no_config)?;
    let defaults = CliDefaults::from_config(&file_config);

    match cli.command {
        CliCommand::Run(args) => run_once::run(args, &defaults).await,
        CliCommand::Batch(args) => batch::batch(args, &defaults).await,
        CliCommand::List(args) => {
            let storage = JsonlStorageReader::new(storage_path(args.output.storage, &defaults));
            let runs = storage.list_page(args.limit, args.offset).await?;
            print_json(&runs, args.output.compact)?;
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
