use crate::{
    args::BatchArgs,
    execute::execute_case,
    input::{prepare_cases, read_batch_cases},
    types::BatchSummary,
};
use anyhow::{anyhow, bail, Context};
use futures::{stream, StreamExt};
use moonlight_core::storage::RunWriter;
use std::time::Instant;
use tokio::sync::mpsc;

pub(crate) async fn batch(args: BatchArgs) -> anyhow::Result<()> {
    if args.quiet && args.emit_runs {
        bail!("--quiet and --emit-runs cannot be used together");
    }

    let jobs = args.jobs.unwrap_or_else(default_jobs).max(1);
    let cases = read_batch_cases(&args.input).await?;
    let prepared_cases = prepare_cases(cases);
    let writer = RunWriter::open(args.storage.storage_path()).await?;
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

    let mut runs = stream::iter(prepared_cases.into_iter().map(|case| {
        let serial_targets = args.serial_targets;
        async move { execute_case(case, serial_targets).await }
    }))
    .buffer_unordered(jobs);

    let mut loop_error = None;
    while let Some(run) = runs.next().await {
        summary.record(&run.comparison.classification);
        if args.emit_runs {
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

    if !args.quiet && !args.emit_runs {
        println!("{}", serde_json::to_string(&summary)?);
    }
    Ok(())
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}
