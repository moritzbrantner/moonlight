mod args;
mod batch;
mod command;
mod config;
mod execute;
mod input;
mod run_once;
mod types;

use anyhow::Context;
use args::{Cli, CliCommand};
use clap::Parser;
use moonlight_core::storage::Storage;

pub async fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommand::Run(args) => run_once::run(args).await,
        CliCommand::Batch(args) => batch::batch(args).await,
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
