pub mod proxy;

use crate::proxy::{
    get_config, get_health, get_metrics, get_run, get_run_report, get_run_review, get_runs,
    get_stats, proxy_handler, put_run_review,
};
use axum::{http::HeaderValue, routing::get, Router};
use moonlight_core::{
    config::AppConfig,
    review::ReviewStore,
    storage::{Storage, StorageOptions},
    Classification, ComparisonRun, MetricsClassificationCounts, MetricsSnapshot,
};
use reqwest::Client;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub client: Client,
    pub storage: Storage,
    pub review_store: ReviewStore,
    pub metrics: AppMetrics,
}

#[derive(Clone, Default)]
pub struct AppMetrics {
    counters: Arc<MetricsCounters>,
}

#[derive(Default)]
struct MetricsCounters {
    total_proxied_comparisons_started: AtomicU64,
    persisted_comparisons: AtomicU64,
    persistence_failures: AtomicU64,
    storage_refresh_failures: AtomicU64,
    target_errors_observed: AtomicU64,
    matches: AtomicU64,
    suspicious_differences: AtomicU64,
    reference_noise: AtomicU64,
    suspicious_with_noise: AtomicU64,
    target_errors: AtomicU64,
}

impl AppMetrics {
    pub fn record_comparison_started(&self) {
        self.counters
            .total_proxied_comparisons_started
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_persisted_run(&self, run: &ComparisonRun) {
        self.counters
            .persisted_comparisons
            .fetch_add(1, Ordering::Relaxed);
        let target_errors = [
            run.primary.error.as_ref(),
            run.candidate.error.as_ref(),
            run.secondary
                .as_ref()
                .and_then(|target| target.error.as_ref()),
        ]
        .into_iter()
        .flatten()
        .count() as u64;
        self.counters
            .target_errors_observed
            .fetch_add(target_errors, Ordering::Relaxed);
        match run.comparison.classification {
            Classification::Match => &self.counters.matches,
            Classification::SuspiciousDifference => &self.counters.suspicious_differences,
            Classification::ReferenceNoise => &self.counters.reference_noise,
            Classification::SuspiciousWithNoise => &self.counters.suspicious_with_noise,
            Classification::TargetError => &self.counters.target_errors,
        }
        .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_persistence_failure(&self) {
        self.counters
            .persistence_failures
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_storage_refresh_failure(&self) {
        self.counters
            .storage_refresh_failures
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_proxied_comparisons_started: self
                .counters
                .total_proxied_comparisons_started
                .load(Ordering::Relaxed),
            persisted_comparisons: self.counters.persisted_comparisons.load(Ordering::Relaxed),
            persistence_failures: self.counters.persistence_failures.load(Ordering::Relaxed),
            storage_refresh_failures: self
                .counters
                .storage_refresh_failures
                .load(Ordering::Relaxed),
            target_errors_observed: self.counters.target_errors_observed.load(Ordering::Relaxed),
            classifications: MetricsClassificationCounts {
                matches: self.counters.matches.load(Ordering::Relaxed),
                suspicious_differences: self
                    .counters
                    .suspicious_differences
                    .load(Ordering::Relaxed),
                reference_noise: self.counters.reference_noise.load(Ordering::Relaxed),
                suspicious_with_noise: self.counters.suspicious_with_noise.load(Ordering::Relaxed),
                target_errors: self.counters.target_errors.load(Ordering::Relaxed),
            },
        }
    }
}

pub async fn build_state(config: AppConfig) -> anyhow::Result<Arc<AppState>> {
    let storage = Storage::load_with_options(
        config.storage_path.clone(),
        StorageOptions {
            retention_max_runs: config.retention_max_runs,
            retention_max_bytes: config.retention_max_bytes,
        },
    )
    .await?;
    let review_store = ReviewStore::load(config.review_state_path.clone()).await?;
    Ok(Arc::new(AppState {
        config,
        client: Client::new(),
        storage,
        review_store,
        metrics: AppMetrics::default(),
    }))
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(get_health))
        .route("/api/config", get(get_config))
        .route("/api/runs", get(get_runs))
        .route("/api/runs/{id}", get(get_run))
        .route("/api/runs/{id}/report", get(get_run_report))
        .route(
            "/api/runs/{id}/review",
            get(get_run_review).put(put_run_review),
        )
        .route("/api/stats", get(get_stats))
        .route("/api/metrics", get(get_metrics))
        .fallback(proxy_handler)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer(&state.config))
        .with_state(state)
}

fn cors_layer(config: &AppConfig) -> CorsLayer {
    if config.cors_origins.iter().any(|origin| origin == "*") {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
    }

    let origins = config
        .cors_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any)
}
