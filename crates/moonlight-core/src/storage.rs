use crate::{Classification, LatencyStats, RequestListItem, RequestRecord, StatsSummary};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::RwLock,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct Storage {
    path: PathBuf,
    records: Arc<RwLock<Vec<RequestRecord>>>,
}

impl Storage {
    pub async fn load(path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut records = Vec::new();
        if fs::try_exists(&path).await? {
            let file = fs::File::open(&path).await?;
            let mut lines = BufReader::new(file).lines();
            while let Some(line) = lines.next_line().await? {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<RequestRecord>(&line) {
                    Ok(record) => records.push(record),
                    Err(error) => eprintln!("skipping corrupt shadowdiff JSONL record: {error}"),
                }
            }
        }

        Ok(Self {
            path,
            records: Arc::new(RwLock::new(records)),
        })
    }

    pub async fn insert(&self, record: RequestRecord) -> anyhow::Result<()> {
        let line = serde_json::to_string(&record)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        self.records.write().await.push(record);
        Ok(())
    }

    pub async fn list(&self) -> Vec<RequestListItem> {
        let records = self.records.read().await;
        records.iter().rev().map(RequestListItem::from).collect()
    }

    pub async fn get(&self, id: Uuid) -> Option<RequestRecord> {
        let records = self.records.read().await;
        records.iter().find(|record| record.id == id).cloned()
    }

    pub async fn stats(&self) -> StatsSummary {
        let records = self.records.read().await;
        let mut matches = 0;
        let mut candidate_diffs = 0;
        let mut noise = 0;
        let mut candidate_diff_with_noise = 0;
        let mut backend_errors = 0;
        let mut candidate_latencies = Vec::new();
        let mut secondary_latencies = Vec::new();
        let mut primary_total = 0_u128;

        for record in records.iter() {
            match record.comparison.classification {
                Classification::Match => matches += 1,
                Classification::CandidateDiff => candidate_diffs += 1,
                Classification::Noise => noise += 1,
                Classification::CandidateDiffWithNoise => candidate_diff_with_noise += 1,
                Classification::BackendError => backend_errors += 1,
            }
            primary_total += record.primary.latency_ms;
            if let Some(candidate) = &record.candidate {
                candidate_latencies.push(candidate.latency_ms);
            }
            if let Some(secondary) = &record.secondary {
                secondary_latencies.push(secondary.latency_ms);
            }
        }

        let total_requests = records.len();
        StatsSummary {
            total_requests,
            matches,
            candidate_diffs,
            noise,
            candidate_diff_with_noise,
            backend_errors,
            latency: LatencyStats {
                primary_avg_ms: avg(total_requests, primary_total),
                candidate_avg_ms: avg_opt(&candidate_latencies),
                secondary_avg_ms: avg_opt(&secondary_latencies),
            },
            latest_requests: records
                .iter()
                .rev()
                .take(20)
                .map(RequestListItem::from)
                .collect(),
        }
    }
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
