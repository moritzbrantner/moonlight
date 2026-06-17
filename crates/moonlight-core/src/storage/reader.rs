use super::{scan::warn_corrupt_line, stats::StatsAccumulator};
use crate::{
    Adapter, Classification, ComparisonRun, ComparisonRunListItem, RunFilter, RunInput, RunPage,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
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
        self.for_each_list_item(|item| {
            accumulator.record_list_item(item);
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

        self.for_each_list_item(|item| {
            if item_matches_filter(&item, filter) {
                total += 1;
                if retained_limit > 0 {
                    runs.push_back(item);
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

    async fn for_each_list_item(
        &self,
        mut visit: impl FnMut(ComparisonRunListItem) -> bool,
    ) -> anyhow::Result<()> {
        self.for_each_line(
            |line| match serde_json::from_str::<StoredRunSummary>(line) {
                Ok(summary) => visit(summary.into()),
                Err(error) => {
                    warn_corrupt_line(&self.path, &error);
                    true
                }
            },
        )
        .await
    }

    async fn for_each_run(
        &self,
        mut visit: impl FnMut(ComparisonRun) -> bool,
    ) -> anyhow::Result<()> {
        self.for_each_line(|line| match serde_json::from_str::<ComparisonRun>(line) {
            Ok(run) => visit(run),
            Err(error) => {
                warn_corrupt_line(&self.path, &error);
                true
            }
        })
        .await
    }

    async fn for_each_line(&self, mut visit: impl FnMut(&str) -> bool) -> anyhow::Result<()> {
        if !self.path.try_exists()? {
            return Ok(());
        }

        let file = std_fs::File::open(&self.path)?;
        let mut reader = StdBufReader::with_capacity(64 * 1024, file);
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line.trim().is_empty() {
                continue;
            }
            if !visit(line) {
                break;
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct StoredRunSummary {
    id: Uuid,
    timestamp: DateTime<Utc>,
    adapter: Adapter,
    input: RunInput,
    primary: StoredTargetSummary,
    candidate: StoredTargetSummary,
    secondary: Option<StoredTargetSummary>,
    comparison: StoredComparisonSummary,
}

#[derive(Deserialize)]
struct StoredTargetSummary {
    status: Option<u16>,
    latency_ms: u128,
}

#[derive(Deserialize)]
struct StoredComparisonSummary {
    classification: Classification,
    #[serde(default)]
    reference_noise: Vec<serde::de::IgnoredAny>,
    #[serde(default)]
    noise_filtered_diffs: Vec<serde::de::IgnoredAny>,
}

impl From<StoredRunSummary> for ComparisonRunListItem {
    fn from(run: StoredRunSummary) -> Self {
        Self {
            id: run.id,
            timestamp: run.timestamp,
            adapter: run.adapter,
            input: run.input,
            primary_status: run.primary.status,
            candidate_status: run.candidate.status,
            secondary_status: run.secondary.as_ref().and_then(|target| target.status),
            classification: run.comparison.classification,
            primary_latency_ms: run.primary.latency_ms,
            candidate_latency_ms: run.candidate.latency_ms,
            secondary_latency_ms: run.secondary.as_ref().map(|target| target.latency_ms),
            diff_count: run.comparison.noise_filtered_diffs.len(),
            noise_count: run.comparison.reference_noise.len(),
        }
    }
}

fn item_matches_filter(item: &ComparisonRunListItem, filter: &RunFilter) -> bool {
    if let Some(classification) = &filter.classification {
        if &item.classification != classification {
            return false;
        }
    }
    if let Some(adapter) = filter.adapter {
        if item.adapter != adapter {
            return false;
        }
    }
    if let Some(status) = filter.status {
        let statuses = [
            item.primary_status,
            item.candidate_status,
            item.secondary_status,
        ];
        if !statuses.into_iter().flatten().any(|value| value == status) {
            return false;
        }
    }
    if let Some(has_noise) = filter.has_noise {
        if (item.noise_count == 0) == has_noise {
            return false;
        }
    }
    if let Some(has_diff) = filter.has_diff {
        if (item.diff_count == 0) == has_diff {
            return false;
        }
    }
    if let Some(query) = &filter.query {
        let query = query.trim().to_ascii_lowercase();
        if !query.is_empty() && !item_search_text(item).contains(&query) {
            return false;
        }
    }
    true
}

fn item_search_text(item: &ComparisonRunListItem) -> String {
    let mut values = vec![
        item.id.to_string(),
        format!("{:?}", item.adapter),
        format!("{:?}", item.classification),
    ];
    match &item.input {
        RunInput::Http {
            method,
            path,
            query,
        } => {
            values.push(method.clone());
            values.push(path.clone());
            if let Some(query) = query {
                values.push(query.clone());
            }
        }
        RunInput::Cli {
            primary_command,
            candidate_command,
            secondary_command,
        } => {
            values.push(primary_command.clone());
            values.push(candidate_command.clone());
            if let Some(command) = secondary_command {
                values.push(command.clone());
            }
        }
        RunInput::Project {
            eval_id,
            project,
            check_id,
            check_name,
            repo,
            baseline_ref,
            candidate_source,
            primary_command,
            candidate_command,
            secondary_command,
        } => {
            values.push(eval_id.to_string());
            values.push(project.clone());
            values.push(check_id.clone());
            if let Some(name) = check_name {
                values.push(name.clone());
            }
            values.push(repo.clone());
            values.push(baseline_ref.clone());
            values.push(candidate_source.clone());
            values.push(primary_command.clone());
            values.push(candidate_command.clone());
            if let Some(command) = secondary_command {
                values.push(command.clone());
            }
        }
    }
    values.join(" ").to_ascii_lowercase()
}
