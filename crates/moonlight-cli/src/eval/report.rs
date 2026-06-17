use anyhow::Context;
use moonlight_core::{ComparisonRun, RunInput};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
use uuid::Uuid;

pub(super) fn read_eval_runs(
    storage_path: &Path,
    eval_id: Uuid,
) -> anyhow::Result<Vec<ComparisonRun>> {
    if !storage_path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(storage_path)
        .with_context(|| format!("failed to read {}", storage_path.display()))?;
    let mut runs = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.with_context(|| format!("failed to read {}", storage_path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let run: ComparisonRun = serde_json::from_str(&line)
            .with_context(|| format!("invalid JSONL in {}", storage_path.display()))?;
        if matches!(&run.input, RunInput::Project { eval_id: id, .. } if *id == eval_id) {
            runs.push(run);
        }
    }
    Ok(runs)
}
