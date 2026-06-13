pub mod compare;
pub mod config;
pub mod storage;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Match,
    CandidateDiff,
    Noise,
    CandidateDiffWithNoise,
    BackendError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Status,
    Header,
    Body,
    BackendError,
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

impl Default for Classification {
    fn default() -> Self {
        Self::Match
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyCapture {
    pub size_bytes: usize,
    pub sha256: String,
    pub preview: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapture {
    pub status: Option<u16>,
    pub headers: BTreeMap<String, String>,
    pub body: BodyCapture,
    pub latency_ms: u128,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub request_headers: BTreeMap<String, String>,
    pub request_body: BodyCapture,
    pub primary: BackendCapture,
    pub candidate: Option<BackendCapture>,
    pub secondary: Option<BackendCapture>,
    pub comparison: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestListItem {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub primary_status: Option<u16>,
    pub candidate_status: Option<u16>,
    pub secondary_status: Option<u16>,
    pub classification: Classification,
    pub primary_latency_ms: u128,
    pub candidate_latency_ms: Option<u128>,
    pub secondary_latency_ms: Option<u128>,
    pub diff_count: usize,
    pub noise_count: usize,
}

impl From<&RequestRecord> for RequestListItem {
    fn from(record: &RequestRecord) -> Self {
        Self {
            id: record.id,
            timestamp: record.timestamp,
            method: record.method.clone(),
            path: record.path.clone(),
            query: record.query.clone(),
            primary_status: record.primary.status,
            candidate_status: record.candidate.as_ref().and_then(|backend| backend.status),
            secondary_status: record.secondary.as_ref().and_then(|backend| backend.status),
            classification: record.comparison.classification.clone(),
            primary_latency_ms: record.primary.latency_ms,
            candidate_latency_ms: record.candidate.as_ref().map(|backend| backend.latency_ms),
            secondary_latency_ms: record.secondary.as_ref().map(|backend| backend.latency_ms),
            diff_count: record.comparison.noise_filtered_diffs.len(),
            noise_count: record.comparison.reference_noise.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub primary_avg_ms: f64,
    pub candidate_avg_ms: Option<f64>,
    pub secondary_avg_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSummary {
    pub total_requests: usize,
    pub matches: usize,
    pub candidate_diffs: usize,
    pub noise: usize,
    pub candidate_diff_with_noise: usize,
    pub backend_errors: usize,
    pub latency: LatencyStats,
    pub latest_requests: Vec<RequestListItem>,
}
