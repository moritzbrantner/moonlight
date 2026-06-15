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
use moonlight_core::storage::JsonlStorageReader;

pub async fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommand::Run(args) => run_once::run(args).await,
        CliCommand::Batch(args) => batch::batch(args).await,
        CliCommand::List(args) => {
            let storage = JsonlStorageReader::new(args.output.storage.storage_path());
            let runs = storage.list_page(args.limit, args.offset).await?;
            print_json(&runs, args.output.compact)?;
            Ok(())
        }
        CliCommand::Stats(args) => {
            let storage = JsonlStorageReader::new(args.storage.storage_path());
            print_json(&storage.stats().await?, args.compact)?;
            Ok(())
        }
        CliCommand::Show(args) => {
            let storage = JsonlStorageReader::new(args.output.storage.storage_path());
            let run = storage
                .get(args.id)
                .await?
                .with_context(|| format!("comparison run {} was not found", args.id))?;
            print_json(&run, args.output.compact)?;
            Ok(())
        }
    }
}

fn print_json(value: &impl serde::Serialize, compact: bool) -> anyhow::Result<()> {
    if compact {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}
