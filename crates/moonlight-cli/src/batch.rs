use crate::{
    args::BatchArgs,
    config::CliDefaults,
    execute::execute_case,
    input::{prepare_cases, read_batch_cases},
    types::BatchSummary,
};
use anyhow::{anyhow, bail, Context};
use futures::{stream, StreamExt};
use moonlight_core::storage::RunWriter;
use std::time::Instant;
use tokio::sync::mpsc;

pub(crate) async fn batch(args: BatchArgs, defaults: &CliDefaults) -> anyhow::Result<()> {
    let quiet = defaults.batch.quiet || args.quiet;
    let emit_runs = defaults.batch.emit_runs || args.emit_runs;
    let serial_targets = defaults.batch.serial_targets || args.serial_targets;
    let jobs = args
        .jobs
        .or(defaults.batch.jobs)
        .unwrap_or_else(default_jobs)
        .max(1);
    let input = args.input.unwrap_or_else(|| defaults.batch.input.clone());
    let storage_path = args
        .storage
        .storage_path
        .unwrap_or_else(|| defaults.storage_path.clone());

    if quiet && emit_runs {
        bail!("--quiet and --emit-runs cannot be used together");
    }

    let cases = read_batch_cases(&input, defaults).await?;
    let prepared_cases = prepare_cases(cases, defaults);
    let writer = RunWriter::open(storage_path).await?;
    let (run_tx, mut run_rx) = mpsc::unbounded_channel();
    let writer_task = tokio::spawn(async move {
        while let Some(run) = run_rx.recv().await {
            writer.append(&run).await?;
        }
        writer.flush().await
    });
    let started = Instant::now();
    let mut summary = BatchSummary {
        jobs,
        ..BatchSummary::default()
    };

    let mut runs = stream::iter(
        prepared_cases
            .into_iter()
            .map(|case| async move { execute_case(case, serial_targets).await }),
    )
    .buffer_unordered(jobs);

    let mut loop_error = None;
    while let Some(run) = runs.next().await {
        summary.record(&run.comparison.classification);
        if emit_runs {
            match serde_json::to_string(&run) {
                Ok(line) => println!("{line}"),
                Err(error) => {
                    loop_error = Some(error.into());
                    break;
                }
            }
        }
        if run_tx.send(run).is_err() {
            loop_error = Some(anyhow!(
                "batch writer stopped before all completed runs were written"
            ));
            break;
        }
    }
    drop(run_tx);
    let writer_result = writer_task
        .await
        .context("batch writer task failed to join")?;
    writer_result.context("batch writer failed")?;
    if let Some(error) = loop_error {
        return Err(error);
    }
    summary.duration_ms = started.elapsed().as_millis();

    if !quiet && !emit_runs {
        println!("{}", serde_json::to_string(&summary)?);
    }
    Ok(())
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}
