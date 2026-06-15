use crate::{Classification, ComparisonRun, ComparisonRunListItem, LatencyStats, StatsSummary};
use std::{
    fs as std_fs,
    io::{BufRead, BufReader as StdBufReader},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader, BufWriter},
    sync::{Mutex, RwLock},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct Storage {
    write_path: PathBuf,
    scan_dir: PathBuf,
    writer: RunWriter,
    options: StorageOptions,
    insert_lock: Arc<Mutex<()>>,
    runs: Arc<RwLock<Vec<ComparisonRun>>>,
}

#[derive(Clone)]
pub struct RunWriter {
    file: Arc<Mutex<BufWriter<File>>>,
}

#[derive(Clone)]
pub struct JsonlStorageReader {
    path: PathBuf,
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

impl JsonlStorageReader {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn stats(&self) -> anyhow::Result<StatsSummary> {
        let mut accumulator = StatsAccumulator::default();
        self.for_each_run(|run| {
            accumulator.record(&run);
            true
        })
        .await?;
        Ok(accumulator.finish())
    }

    pub async fn list_page(
        &self,
        limit: Option<usize>,
        offset: usize,
    ) -> anyhow::Result<Vec<ComparisonRunListItem>> {
        let retained_limit = limit.and_then(|value| value.checked_add(offset));
        let mut runs = Vec::new();

        self.for_each_run(|run| {
            runs.push(ComparisonRunListItem::from(&run));
            if let Some(retained_limit) = retained_limit {
                if runs.len() > retained_limit {
                    runs.remove(0);
                }
            }
            true
        })
        .await?;

        runs.reverse();
        Ok(match limit {
            Some(limit) => runs.into_iter().skip(offset).take(limit).collect(),
            None => runs.into_iter().skip(offset).collect(),
        })
    }

    pub async fn get(&self, id: Uuid) -> anyhow::Result<Option<ComparisonRun>> {
        let mut found = None;
        self.for_each_run(|run| {
            if run.id == id {
                found = Some(run);
                false
            } else {
                true
            }
        })
        .await?;
        Ok(found)
    }

    async fn for_each_run(
        &self,
        mut visit: impl FnMut(ComparisonRun) -> bool,
    ) -> anyhow::Result<()> {
        if !self.path.try_exists()? {
            return Ok(());
        }

        let file = std_fs::File::open(&self.path)?;
        let lines = StdBufReader::new(file).lines();
        for line in lines {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ComparisonRun>(&line) {
                Ok(run) => {
                    if !visit(run) {
                        break;
                    }
                }
                Err(error) => warn_corrupt_line(&self.path, &error),
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StorageOptions {
    pub retention_max_runs: Option<usize>,
    pub retention_max_bytes: Option<u64>,
}

impl Storage {
    pub async fn load(write_path: PathBuf) -> anyhow::Result<Self> {
        Self::load_with_options(write_path, StorageOptions::default()).await
    }

    pub async fn load_with_options(
        write_path: PathBuf,
        options: StorageOptions,
    ) -> anyhow::Result<Self> {
        if let Some(parent) = write_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let scan_dir = write_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let runs = load_runs_from_dir(&scan_dir).await?;
        let writer = RunWriter::open(write_path.clone()).await?;

        Ok(Self {
            write_path,
            scan_dir,
            writer,
            options,
            insert_lock: Arc::new(Mutex::new(())),
            runs: Arc::new(RwLock::new(runs)),
        })
    }

    pub async fn insert(&self, run: ComparisonRun) -> anyhow::Result<()> {
        let _guard = self.insert_lock.lock().await;
        self.writer.append(&run).await?;
        self.writer.flush().await?;
        self.runs.write().await.push(run);
        self.apply_retention().await?;
        Ok(())
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        let runs = load_runs_from_dir(&self.scan_dir).await?;
        *self.runs.write().await = runs;
        Ok(())
    }

    pub async fn list(&self) -> Vec<ComparisonRunListItem> {
        self.list_page(usize::MAX, 0).await
    }

    pub async fn list_page(&self, limit: usize, offset: usize) -> Vec<ComparisonRunListItem> {
        let runs = self.runs.read().await;
        runs.iter()
            .rev()
            .skip(offset)
            .take(limit)
            .map(ComparisonRunListItem::from)
            .collect()
    }

    pub async fn get(&self, id: Uuid) -> Option<ComparisonRun> {
        let runs = self.runs.read().await;
        runs.iter().find(|run| run.id == id).cloned()
    }

    pub async fn stats(&self) -> StatsSummary {
        let runs = self.runs.read().await;
        let mut accumulator = StatsAccumulator::default();

        for run in runs.iter() {
            accumulator.record(run);
        }

        accumulator.finish()
    }

    async fn apply_retention(&self) -> anyhow::Result<()> {
        if self.options.retention_max_runs.is_none() && self.options.retention_max_bytes.is_none() {
            return Ok(());
        }

        let mut active_runs = Vec::new();
        load_runs_from_file(&self.write_path, &mut active_runs).await?;
        active_runs.sort_by_key(|run| run.timestamp);

        if let Some(max_runs) = self.options.retention_max_runs {
            if active_runs.len() > max_runs {
                active_runs = active_runs
                    .into_iter()
                    .rev()
                    .take(max_runs)
                    .collect::<Vec<_>>();
                active_runs.reverse();
            }
        }

        if let Some(max_bytes) = self.options.retention_max_bytes {
            let mut retained = Vec::new();
            let mut total_bytes = 0_u64;
            for run in active_runs.into_iter().rev() {
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
            active_runs = retained;
        }

        let mut content = String::new();
        for run in active_runs {
            content.push_str(&serde_json::to_string(&run)?);
            content.push('\n');
        }
        fs::write(&self.write_path, content).await?;
        self.refresh().await?;
        Ok(())
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

fn warn_corrupt_line(path: &Path, error: &serde_json::Error) {
    eprintln!(
        "skipping corrupt moonlight JSONL run in {}: {error}",
        path.display()
    );
}

#[derive(Default)]
struct StatsAccumulator {
    total_runs: usize,
    matches: usize,
    suspicious_differences: usize,
    reference_noise: usize,
    suspicious_with_noise: usize,
    target_errors: usize,
    primary_total: u128,
    candidate_total: u128,
    secondary_latencies: Vec<u128>,
    latest_runs: Vec<ComparisonRunListItem>,
}

impl StatsAccumulator {
    fn record(&mut self, run: &ComparisonRun) {
        self.total_runs += 1;
        match run.comparison.classification {
            Classification::Match => self.matches += 1,
            Classification::SuspiciousDifference => self.suspicious_differences += 1,
            Classification::ReferenceNoise => self.reference_noise += 1,
            Classification::SuspiciousWithNoise => self.suspicious_with_noise += 1,
            Classification::TargetError => self.target_errors += 1,
        }
        self.primary_total += run.primary.latency_ms;
        self.candidate_total += run.candidate.latency_ms;
        if let Some(secondary) = &run.secondary {
            self.secondary_latencies.push(secondary.latency_ms);
        }

        self.latest_runs.push(ComparisonRunListItem::from(run));
        if self.latest_runs.len() > 20 {
            self.latest_runs.remove(0);
        }
    }

    fn finish(mut self) -> StatsSummary {
        self.latest_runs.reverse();
        StatsSummary {
            total_runs: self.total_runs,
            matches: self.matches,
            suspicious_differences: self.suspicious_differences,
            reference_noise: self.reference_noise,
            suspicious_with_noise: self.suspicious_with_noise,
            target_errors: self.target_errors,
            latency: LatencyStats {
                primary_avg_ms: avg(self.total_runs, self.primary_total),
                candidate_avg_ms: avg(self.total_runs, self.candidate_total),
                secondary_avg_ms: avg_opt(&self.secondary_latencies),
            },
            latest_runs: self.latest_runs,
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

#[cfg(test)]
mod tests;
