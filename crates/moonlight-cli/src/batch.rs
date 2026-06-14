use crate::{
    args::BatchArgs,
    execute::execute_case,
    input::{prepare_cases, read_batch_cases},
    types::BatchSummary,
};
use anyhow::bail;
use futures::{stream, StreamExt};
use moonlight_core::storage::RunWriter;
use std::{sync::Arc, time::Instant};

pub(crate) async fn batch(args: BatchArgs) -> anyhow::Result<()> {
    if args.quiet && args.emit_runs {
        bail!("--quiet and --emit-runs cannot be used together");
    }

    let jobs = args.jobs.unwrap_or_else(default_jobs).max(1);
    let cases = read_batch_cases(&args.input).await?;
    let prepared_cases = prepare_cases(cases);
    let writer = RunWriter::open(args.storage.storage_path).await?;
    let writer = Arc::new(writer);
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

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}
