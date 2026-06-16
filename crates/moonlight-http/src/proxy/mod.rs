mod api;
mod execution;
mod target;

use axum::http::{HeaderMap, Method};
use bytes::Bytes;
use chrono::Utc;
use moonlight_core::BodyCapture;
use std::collections::BTreeMap;
use uuid::Uuid;

pub use api::{
    get_config, get_health, get_metrics, get_run, get_run_report, get_run_review, get_runs,
    get_stats, put_run_review,
};
pub use execution::proxy_handler;

struct RunMetadata {
    id: Uuid,
    timestamp: chrono::DateTime<Utc>,
    method: String,
    path: String,
    query: Option<String>,
    request_headers: BTreeMap<String, String>,
    request_body: BodyCapture,
}

#[derive(Clone)]
struct TargetRequest {
    method: Method,
    path_and_query: String,
    headers: HeaderMap,
    body: Bytes,
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
