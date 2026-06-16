pub mod proxy;

use crate::proxy::{
    get_config, get_health, get_run, get_run_report, get_run_review, get_runs, get_stats,
    proxy_handler, put_run_review,
};
use axum::{http::HeaderValue, routing::get, Router};
use moonlight_core::{
    config::AppConfig,
    review::ReviewStore,
    storage::{Storage, StorageOptions},
};
use reqwest::Client;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub client: Client,
    pub storage: Storage,
    pub review_store: ReviewStore,
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
        .fallback(proxy_handler)
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
