mod api;
mod execution;
mod target;

use axum::http::{HeaderMap, Method};
use bytes::Bytes;

pub use api::{
    get_config, get_health, get_metrics, get_run, get_run_report, get_run_review, get_runs,
    get_stats, put_run_review,
};
pub use execution::proxy_handler;

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
