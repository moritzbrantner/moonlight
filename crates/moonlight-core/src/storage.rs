use crate::{Classification, ComparisonRun, ComparisonRunListItem, LatencyStats, StatsSummary};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::RwLock,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct Storage {
    write_path: PathBuf,
    scan_dir: PathBuf,
    runs: Arc<RwLock<Vec<ComparisonRun>>>,
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
        let line = serde_json::to_string(&run)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.write_path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
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
mod tests {
    use super::*;
    use crate::{
        Adapter, BodyCapture, Classification, ComparisonRun, ComparisonSummary, RunInput,
        TargetObservation,
    };
    use chrono::{TimeZone, Utc};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn body() -> BodyCapture {
        BodyCapture {
            size_bytes: 0,
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            preview: String::new(),
            truncated: false,
        }
    }

    fn target(latency_ms: u128) -> TargetObservation {
        TargetObservation {
            status: Some(0),
            headers: BTreeMap::new(),
            body: body(),
            stderr: None,
            latency_ms,
            error: None,
        }
    }

    fn run(
        path: impl Into<String>,
        timestamp_seconds: i64,
        classification: Classification,
        secondary: bool,
    ) -> ComparisonRun {
        let path = path.into();
        ComparisonRun {
            id: Uuid::new_v4(),
            timestamp: Utc.timestamp_opt(timestamp_seconds, 0).unwrap(),
            adapter: Adapter::Http,
            input: RunInput::Http {
                method: "GET".to_string(),
                path,
                query: None,
            },
            request_headers: BTreeMap::new(),
            request_body: body(),
            primary: target(10),
            candidate: target(20),
            secondary: secondary.then(|| target(30)),
            comparison: ComparisonSummary {
                classification,
                ..Default::default()
            },
        }
    }

    #[tokio::test]
    async fn load_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("http-runs.jsonl");

        let _storage = Storage::load(path.clone()).await.unwrap();

        assert!(path.parent().unwrap().exists());
    }

    #[tokio::test]
    async fn load_skips_empty_and_corrupt_jsonl_lines() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("http-runs.jsonl");
        let valid = serde_json::to_string(&run("valid", 1, Classification::Match, false)).unwrap();
        std::fs::write(&path, format!("\n{valid}\nnot-json\n\n")).unwrap();

        let storage = Storage::load(path).await.unwrap();
        let runs = storage.list().await;

        assert_eq!(runs.len(), 1);
        assert!(matches!(
            runs[0].input,
            RunInput::Http { ref path, .. } if path == "valid"
        ));
    }

    #[tokio::test]
    async fn list_returns_newest_first() {
        let dir = tempdir().unwrap();
        let storage = Storage::load(dir.path().join("http-runs.jsonl"))
            .await
            .unwrap();
        let first = run("first", 1, Classification::Match, false);
        let second = run("second", 2, Classification::SuspiciousDifference, false);
        storage.insert(first).await.unwrap();
        storage.insert(second).await.unwrap();

        let runs = storage.list().await;

        assert!(matches!(
            runs[0].input,
            RunInput::Http { ref path, .. } if path == "second"
        ));
        assert!(matches!(
            runs[1].input,
            RunInput::Http { ref path, .. } if path == "first"
        ));
    }

    #[tokio::test]
    async fn load_merges_jsonl_files_in_same_directory() {
        let dir = tempdir().unwrap();
        let http_path = dir.path().join("http-runs.jsonl");
        let cli_path = dir.path().join("cli-runs.jsonl");
        std::fs::write(
            &http_path,
            format!(
                "{}\n",
                serde_json::to_string(&run("http", 1, Classification::Match, false)).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(
            &cli_path,
            format!(
                "{}\n",
                serde_json::to_string(&run("cli", 2, Classification::ReferenceNoise, true))
                    .unwrap()
            ),
        )
        .unwrap();

        let storage = Storage::load(http_path).await.unwrap();
        let stats = storage.stats().await;

        assert_eq!(stats.total_runs, 2);
        assert_eq!(stats.matches, 1);
        assert_eq!(stats.reference_noise, 1);
    }

    #[tokio::test]
    async fn stats_limits_latest_runs_to_20() {
        let dir = tempdir().unwrap();
        let storage = Storage::load(dir.path().join("http-runs.jsonl"))
            .await
            .unwrap();
        for index in 0..25 {
            storage
                .insert(run(
                    format!("run-{index}"),
                    index,
                    Classification::Match,
                    false,
                ))
                .await
                .unwrap();
        }

        let stats = storage.stats().await;

        assert_eq!(stats.total_runs, 25);
        assert_eq!(stats.latest_runs.len(), 20);
        assert!(matches!(
            stats.latest_runs[0].input,
            RunInput::Http { ref path, .. } if path == "run-24"
        ));
        assert!(matches!(
            stats.latest_runs[19].input,
            RunInput::Http { ref path, .. } if path == "run-5"
        ));
    }

    #[tokio::test]
    async fn stats_handles_missing_secondary_latencies() {
        let dir = tempdir().unwrap();
        let storage = Storage::load(dir.path().join("http-runs.jsonl"))
            .await
            .unwrap();
        storage
            .insert(run("primary-candidate", 1, Classification::Match, false))
            .await
            .unwrap();

        let stats = storage.stats().await;

        assert_eq!(stats.total_runs, 1);
        assert_eq!(stats.latency.primary_avg_ms, 10.0);
        assert_eq!(stats.latency.candidate_avg_ms, 20.0);
        assert_eq!(stats.latency.secondary_avg_ms, None);
    }
}
