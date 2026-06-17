mod reader;
mod retention;
mod scan;
mod stats;
mod writer;

use crate::{run_matches_filter, ComparisonRun, ComparisonRunListItem, RunFilter, RunPage};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs,
    sync::{Mutex, RwLock},
};
use uuid::Uuid;

pub use reader::JsonlStorageReader;
pub use retention::StorageOptions;
use retention::{atomic_write, retain_runs, serialize_runs_jsonl};
use scan::{load_runs_from_file, load_runs_from_signature, scan_jsonl_files, JsonlFileSignature};
use stats::StatsAccumulator;
pub use writer::RunWriter;

#[derive(Clone)]
pub struct Storage {
    write_path: PathBuf,
    scan_dir: PathBuf,
    writer: RunWriter,
    options: StorageOptions,
    insert_lock: Arc<Mutex<()>>,
    runs: Arc<RwLock<Vec<ComparisonRun>>>,
    scan_signature: Arc<Mutex<Vec<JsonlFileSignature>>>,
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
        let scan_signature = scan_jsonl_files(&scan_dir).await?;
        let runs = load_runs_from_signature(&scan_signature).await?;
        let writer = RunWriter::open(write_path.clone()).await?;

        Ok(Self {
            write_path,
            scan_dir,
            writer,
            options,
            insert_lock: Arc::new(Mutex::new(())),
            runs: Arc::new(RwLock::new(runs)),
            scan_signature: Arc::new(Mutex::new(scan_signature)),
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

    pub async fn refresh(&self) -> anyhow::Result<bool> {
        let scan_signature = scan_jsonl_files(&self.scan_dir).await?;
        {
            let current = self.scan_signature.lock().await;
            if *current == scan_signature {
                return Ok(false);
            }
        }

        let runs = load_runs_from_signature(&scan_signature).await?;
        *self.runs.write().await = runs;
        *self.scan_signature.lock().await = scan_signature;
        Ok(true)
    }

    pub async fn list(&self) -> Vec<ComparisonRunListItem> {
        self.list_page(usize::MAX, 0).await
    }

    pub async fn list_page(&self, limit: usize, offset: usize) -> Vec<ComparisonRunListItem> {
        self.filtered_page(&RunFilter::default(), limit, offset)
            .await
            .items
    }

    pub async fn filtered_page(&self, filter: &RunFilter, limit: usize, offset: usize) -> RunPage {
        let runs = self.runs.read().await;
        let mut total = 0;
        let mut items = Vec::new();

        for run in runs
            .iter()
            .rev()
            .filter(|run| run_matches_filter(run, filter))
        {
            if total >= offset && items.len() < limit {
                items.push(ComparisonRunListItem::from(run));
            }
            total += 1;
        }

        let next_offset = (offset + items.len() < total).then_some(offset + items.len());
        RunPage {
            items,
            limit,
            offset,
            total,
            next_offset,
        }
    }

    pub async fn get(&self, id: Uuid) -> Option<ComparisonRun> {
        let runs = self.runs.read().await;
        runs.iter().find(|run| run.id == id).cloned()
    }

    pub async fn stats(&self) -> crate::StatsSummary {
        let runs = self.runs.read().await;
        let mut accumulator = StatsAccumulator::default();

        for run in runs.iter() {
            accumulator.record(run);
        }

        accumulator.finish()
    }

    async fn apply_retention(&self) -> anyhow::Result<()> {
        if !self.options.is_configured() {
            return Ok(());
        }

        let mut active_runs = Vec::new();
        load_runs_from_file(&self.write_path, &mut active_runs).await?;
        active_runs.sort_by_key(|run| run.timestamp);

        let retained_runs = retain_runs(active_runs.clone(), self.options)?;
        let active_content = serialize_runs_jsonl(&active_runs)?;
        let retained_content = serialize_runs_jsonl(&retained_runs)?;
        if active_content == retained_content {
            return Ok(());
        }

        atomic_write(&self.write_path, retained_content).await?;
        self.writer.reopen(&self.write_path).await?;
        self.force_refresh().await?;
        Ok(())
    }

    async fn force_refresh(&self) -> anyhow::Result<()> {
        let scan_signature = scan_jsonl_files(&self.scan_dir).await?;
        let runs = load_runs_from_signature(&scan_signature).await?;
        *self.runs.write().await = runs;
        *self.scan_signature.lock().await = scan_signature;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
