use crate::ComparisonRun;
use std::path::Path;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct StorageOptions {
    pub retention_max_runs: Option<usize>,
    pub retention_max_bytes: Option<u64>,
}

impl StorageOptions {
    pub(super) fn is_configured(self) -> bool {
        self.retention_max_runs.is_some() || self.retention_max_bytes.is_some()
    }
}

pub(super) fn retain_runs(
    mut runs: Vec<ComparisonRun>,
    options: StorageOptions,
) -> anyhow::Result<Vec<ComparisonRun>> {
    if let Some(max_runs) = options.retention_max_runs {
        runs = retain_by_max_runs(runs, max_runs);
    }
    if let Some(max_bytes) = options.retention_max_bytes {
        runs = retain_by_max_bytes(runs, max_bytes)?;
    }
    Ok(runs)
}

fn retain_by_max_runs(runs: Vec<ComparisonRun>, max_runs: usize) -> Vec<ComparisonRun> {
    if runs.len() <= max_runs {
        return runs;
    }

    let mut retained = runs.into_iter().rev().take(max_runs).collect::<Vec<_>>();
    retained.reverse();
    retained
}

fn retain_by_max_bytes(
    runs: Vec<ComparisonRun>,
    max_bytes: u64,
) -> anyhow::Result<Vec<ComparisonRun>> {
    let mut retained = Vec::new();
    let mut total_bytes = 0_u64;
    for run in runs.into_iter().rev() {
        let line = serde_json::to_string(&run)?;
        let line_bytes = line.len() as u64 + 1;
        if total_bytes + line_bytes <= max_bytes || retained.is_empty() {
            total_bytes += line_bytes;
            retained.push(run);
        } else {
            break;
        }
    }
    retained.reverse();
    Ok(retained)
}

pub(super) fn serialize_runs_jsonl(runs: &[ComparisonRun]) -> anyhow::Result<String> {
    let mut content = String::new();
    for run in runs {
        content.push_str(&serde_json::to_string(run)?);
        content.push('\n');
    }
    Ok(content)
}

pub(super) async fn atomic_write(path: &Path, content: String) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("moonlight-runs.jsonl");
    let temp_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        Uuid::new_v4()
    ));

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await?;
    file.sync_data().await?;
    drop(file);
    fs::rename(&temp_path, path).await?;
    Ok(())
}
