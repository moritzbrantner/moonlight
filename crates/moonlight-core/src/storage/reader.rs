use super::{scan::warn_corrupt_line, stats::StatsAccumulator};
use crate::{run_matches_filter, ComparisonRun, ComparisonRunListItem, RunFilter, RunPage};
use std::{
    collections::VecDeque,
    fs as std_fs,
    io::{BufRead, BufReader as StdBufReader},
    path::PathBuf,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct JsonlStorageReader {
    path: PathBuf,
}

impl JsonlStorageReader {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn stats(&self) -> anyhow::Result<crate::StatsSummary> {
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
        Ok(self
            .filtered_page(&RunFilter::default(), limit.unwrap_or(usize::MAX), offset)
            .await?
            .items)
    }

    pub async fn filtered_page(
        &self,
        filter: &RunFilter,
        limit: usize,
        offset: usize,
    ) -> anyhow::Result<RunPage> {
        let retained_limit = limit.saturating_add(offset);
        let mut runs = VecDeque::new();
        let mut total = 0;

        self.for_each_run(|run| {
            if run_matches_filter(&run, filter) {
                total += 1;
                if retained_limit > 0 {
                    runs.push_back(ComparisonRunListItem::from(&run));
                    if runs.len() > retained_limit {
                        runs.pop_front();
                    }
                }
            }
            true
        })
        .await?;

        let items = runs
            .into_iter()
            .rev()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        let next_offset = (offset + items.len() < total).then_some(offset + items.len());
        Ok(RunPage {
            items,
            limit,
            offset,
            total,
            next_offset,
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
