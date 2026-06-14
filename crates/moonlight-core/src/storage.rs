use crate::{Classification, ComparisonRun, ComparisonRunListItem, LatencyStats, StatsSummary};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    sync::{Mutex, RwLock},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct Storage {
    write_path: PathBuf,
    scan_dir: PathBuf,
    runs: Arc<RwLock<Vec<ComparisonRun>>>,
}

#[derive(Clone)]
pub struct RunWriter {
    file: Arc<Mutex<BufWriter<File>>>,
}

impl RunWriter {
    pub async fn open(write_path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = write_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(write_path)
            .await?;

        Ok(Self {
            file: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }

    pub async fn append(&self, run: &ComparisonRun) -> anyhow::Result<()> {
        let line = serde_json::to_string(run)?;
        let mut file = self.file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn flush(&self) -> anyhow::Result<()> {
        self.file.lock().await.flush().await?;
        Ok(())
    }
}

impl Storage {
    pub async fn load(write_path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = write_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let scan_dir = write_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let runs = load_runs_from_dir(&scan_dir).await?;

        Ok(Self {
            write_path,
            scan_dir,
            runs: Arc::new(RwLock::new(runs)),
        })
    }

    pub async fn insert(&self, run: ComparisonRun) -> anyhow::Result<()> {
        let writer = RunWriter::open(self.write_path.clone()).await?;
        writer.append(&run).await?;
        writer.flush().await?;
        self.runs.write().await.push(run);
        Ok(())
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        let runs = load_runs_from_dir(&self.scan_dir).await?;
        *self.runs.write().await = runs;
        Ok(())
    }

    pub async fn list(&self) -> Vec<ComparisonRunListItem> {
        let runs = self.runs.read().await;
        runs.iter().rev().map(ComparisonRunListItem::from).collect()
    }

    pub async fn get(&self, id: Uuid) -> Option<ComparisonRun> {
        let runs = self.runs.read().await;
        runs.iter().find(|run| run.id == id).cloned()
    }

    pub async fn stats(&self) -> StatsSummary {
        let runs = self.runs.read().await;
        let mut matches = 0;
        let mut suspicious_differences = 0;
        let mut reference_noise = 0;
        let mut suspicious_with_noise = 0;
        let mut target_errors = 0;
        let mut primary_total = 0_u128;
        let mut candidate_total = 0_u128;
        let mut secondary_latencies = Vec::new();

        for run in runs.iter() {
            match run.comparison.classification {
                Classification::Match => matches += 1,
                Classification::SuspiciousDifference => suspicious_differences += 1,
                Classification::ReferenceNoise => reference_noise += 1,
                Classification::SuspiciousWithNoise => suspicious_with_noise += 1,
                Classification::TargetError => target_errors += 1,
            }
            primary_total += run.primary.latency_ms;
            candidate_total += run.candidate.latency_ms;
            if let Some(secondary) = &run.secondary {
                secondary_latencies.push(secondary.latency_ms);
            }
        }

        let total_runs = runs.len();
        StatsSummary {
            total_runs,
            matches,
            suspicious_differences,
            reference_noise,
            suspicious_with_noise,
            target_errors,
            latency: LatencyStats {
                primary_avg_ms: avg(total_runs, primary_total),
                candidate_avg_ms: avg(total_runs, candidate_total),
                secondary_avg_ms: avg_opt(&secondary_latencies),
            },
            latest_runs: runs
                .iter()
                .rev()
                .take(20)
                .map(ComparisonRunListItem::from)
                .collect(),
        }
    }
}

async fn load_runs_from_dir(scan_dir: &Path) -> anyhow::Result<Vec<ComparisonRun>> {
    let mut runs = Vec::new();
    if !fs::try_exists(scan_dir).await? {
        return Ok(runs);
    }

    let mut entries = fs::read_dir(scan_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        load_runs_from_file(&path, &mut runs).await?;
    }

    runs.sort_by_key(|run| run.timestamp);
    Ok(runs)
}

async fn load_runs_from_file(path: &Path, runs: &mut Vec<ComparisonRun>) -> anyhow::Result<()> {
    let file = fs::File::open(path).await?;
    let mut lines = BufReader::new(file).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<ComparisonRun>(&line) {
            Ok(run) => runs.push(run),
            Err(error) => eprintln!(
                "skipping corrupt moonlight JSONL run in {}: {error}",
                path.display()
            ),
        }
    }
    Ok(())
}

fn avg(count: usize, total: u128) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

fn avg_opt(values: &[u128]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<u128>() as f64 / values.len() as f64)
    }
}

#[cfg(test)]
mod tests;
