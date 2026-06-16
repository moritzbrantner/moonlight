pub mod compare;
pub mod config;
pub mod report;
pub mod review;
pub mod storage;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    #[default]
    Match,
    SuspiciousDifference,
    ReferenceNoise,
    SuspiciousWithNoise,
    TargetError,
}

impl std::str::FromStr for Classification {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "match" => Ok(Self::Match),
            "suspicious_difference" => Ok(Self::SuspiciousDifference),
            "reference_noise" => Ok(Self::ReferenceNoise),
            "suspicious_with_noise" => Ok(Self::SuspiciousWithNoise),
            "target_error" => Ok(Self::TargetError),
            other => anyhow::bail!("invalid classification {other:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Status,
    Header,
    Body,
    Stderr,
    TargetError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffEntry {
    pub kind: DiffKind,
    pub path: String,
    pub primary: Option<String>,
    pub candidate: Option<String>,
    pub secondary: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComparisonSummary {
    pub classification: Classification,
    pub raw_candidate_diffs: Vec<DiffEntry>,
    pub reference_noise: Vec<DiffEntry>,
    pub noise_filtered_diffs: Vec<DiffEntry>,
    pub raw_diff_summary: String,
    pub noise_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyCapture {
    pub size_bytes: usize,
    pub sha256: String,
    pub preview: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetObservation {
    pub status: Option<u16>,
    pub headers: BTreeMap<String, String>,
    pub body: BodyCapture,
    pub stderr: Option<BodyCapture>,
    pub latency_ms: u128,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Adapter {
    Http,
    Cli,
}

impl std::str::FromStr for Adapter {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "http" => Ok(Self::Http),
            "cli" => Ok(Self::Cli),
            other => anyhow::bail!("invalid adapter {other:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunInput {
    Http {
        method: String,
        path: String,
        query: Option<String>,
    },
    Cli {
        primary_command: String,
        candidate_command: String,
        secondary_command: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonRun {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub adapter: Adapter,
    pub input: RunInput,
    pub request_headers: BTreeMap<String, String>,
    pub request_body: BodyCapture,
    pub primary: TargetObservation,
    pub candidate: TargetObservation,
    pub secondary: Option<TargetObservation>,
    pub comparison: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonRunListItem {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub adapter: Adapter,
    pub input: RunInput,
    pub primary_status: Option<u16>,
    pub candidate_status: Option<u16>,
    pub secondary_status: Option<u16>,
    pub classification: Classification,
    pub primary_latency_ms: u128,
    pub candidate_latency_ms: u128,
    pub secondary_latency_ms: Option<u128>,
    pub diff_count: usize,
    pub noise_count: usize,
}

impl From<&ComparisonRun> for ComparisonRunListItem {
    fn from(run: &ComparisonRun) -> Self {
        Self {
            id: run.id,
            timestamp: run.timestamp,
            adapter: run.adapter,
            input: run.input.clone(),
            primary_status: run.primary.status,
            candidate_status: run.candidate.status,
            secondary_status: run.secondary.as_ref().and_then(|target| target.status),
            classification: run.comparison.classification.clone(),
            primary_latency_ms: run.primary.latency_ms,
            candidate_latency_ms: run.candidate.latency_ms,
            secondary_latency_ms: run.secondary.as_ref().map(|target| target.latency_ms),
            diff_count: run.comparison.noise_filtered_diffs.len(),
            noise_count: run.comparison.reference_noise.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub primary_avg_ms: f64,
    pub candidate_avg_ms: f64,
    pub secondary_avg_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSummary {
    pub total_runs: usize,
    pub matches: usize,
    pub suspicious_differences: usize,
    pub reference_noise: usize,
    pub suspicious_with_noise: usize,
    pub target_errors: usize,
    pub latency: LatencyStats,
    pub latest_runs: Vec<ComparisonRunListItem>,
}

#[derive(Debug, Clone, Default)]
pub struct RunFilter {
    pub classification: Option<Classification>,
    pub adapter: Option<Adapter>,
    pub query: Option<String>,
    pub status: Option<u16>,
    pub has_noise: Option<bool>,
    pub has_diff: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunPage {
    pub items: Vec<ComparisonRunListItem>,
    pub limit: usize,
    pub offset: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
}

pub fn run_matches_filter(run: &ComparisonRun, filter: &RunFilter) -> bool {
    if let Some(classification) = &filter.classification {
        if &run.comparison.classification != classification {
            return false;
        }
    }
    if let Some(adapter) = filter.adapter {
        if run.adapter != adapter {
            return false;
        }
    }
    if let Some(status) = filter.status {
        let statuses = [
            run.primary.status,
            run.candidate.status,
            run.secondary.as_ref().and_then(|target| target.status),
        ];
        if !statuses.into_iter().flatten().any(|value| value == status) {
            return false;
        }
    }
    if let Some(has_noise) = filter.has_noise {
        if run.comparison.reference_noise.is_empty() == has_noise {
            return false;
        }
    }
    if let Some(has_diff) = filter.has_diff {
        if run.comparison.noise_filtered_diffs.is_empty() == has_diff {
            return false;
        }
    }
    if let Some(query) = &filter.query {
        let query = query.trim().to_ascii_lowercase();
        if !query.is_empty() && !run_search_text(run).contains(&query) {
            return false;
        }
    }
    true
}

fn run_search_text(run: &ComparisonRun) -> String {
    let mut values = vec![
        run.id.to_string(),
        format!("{:?}", run.adapter),
        format!("{:?}", run.comparison.classification),
    ];
    match &run.input {
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
    }
    values.join(" ").to_ascii_lowercase()
}
