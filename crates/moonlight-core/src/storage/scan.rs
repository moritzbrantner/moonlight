use crate::ComparisonRun;
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader as TokioBufReader},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct JsonlFileSignature {
    pub path: PathBuf,
    len: u64,
    modified: Option<SystemTime>,
}

pub(super) async fn load_runs_from_signature(
    signature: &[JsonlFileSignature],
) -> anyhow::Result<Vec<ComparisonRun>> {
    let mut runs = Vec::new();
    for file in signature {
        load_runs_from_file(&file.path, &mut runs).await?;
    }
    runs.sort_by_key(|run| run.timestamp);
    Ok(runs)
}

pub(super) async fn scan_jsonl_files(scan_dir: &Path) -> anyhow::Result<Vec<JsonlFileSignature>> {
    let mut signature = Vec::new();
    if !fs::try_exists(scan_dir).await? {
        return Ok(signature);
    }

    let mut entries = fs::read_dir(scan_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let metadata = entry.metadata().await?;
        signature.push(JsonlFileSignature {
            path,
            len: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }

    signature.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(signature)
}

pub(super) async fn load_runs_from_file(
    path: &Path,
    runs: &mut Vec<ComparisonRun>,
) -> anyhow::Result<()> {
    let file = fs::File::open(path).await?;
    let mut lines = TokioBufReader::new(file).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<ComparisonRun>(&line) {
            Ok(run) => runs.push(run),
            Err(error) => warn_corrupt_line(path, &error),
        }
    }
    Ok(())
}

pub(super) fn warn_corrupt_line(path: &Path, error: &serde_json::Error) {
    tracing::warn!(
        path = %path.display(),
        error = %error,
        "skipping corrupt moonlight JSONL run in {}: {error}",
        path.display()
    );
}
