pub mod compare;
pub mod config;
pub mod proxy;
pub mod storage;

use crate::config::AppConfig;
use crate::proxy::{get_config, get_health, get_request, get_requests, get_stats, proxy_handler};
use crate::storage::Storage;
use axum::{routing::get, Router};
use reqwest::Client;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub client: Client,
    pub storage: Storage,
}

pub async fn build_state(config: AppConfig) -> anyhow::Result<Arc<AppState>> {
    let storage = Storage::load(config.storage_path.clone()).await?;
    Ok(Arc::new(AppState {
        config,
        client: Client::new(),
        storage,
    }))
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(get_health))
        .route("/api/config", get(get_config))
        .route("/api/requests", get(get_requests))
        .route("/api/requests/:id", get(get_request))
        .route("/api/stats", get(get_stats))
        .fallback(proxy_handler)
        .layer(CorsLayer::permissive())
        .with_state(state)
}
