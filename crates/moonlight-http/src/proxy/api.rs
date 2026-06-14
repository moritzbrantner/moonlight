use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use moonlight_core::ComparisonRun;
use std::sync::Arc;
use uuid::Uuid;

pub async fn get_health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

pub async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.config.clone())
}

pub async fn get_runs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    refresh_storage(&state).await;
    Json(state.storage.list().await)
}

pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ComparisonRun>, StatusCode> {
    refresh_storage(&state).await;
    state
        .storage
        .get(id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    refresh_storage(&state).await;
    Json(state.storage.stats().await)
}

async fn refresh_storage(state: &AppState) {
    if let Err(error) = state.storage.refresh().await {
        eprintln!("failed to refresh moonlight run storage: {error}");
    }
}
