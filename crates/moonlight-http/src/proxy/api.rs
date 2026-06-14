use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use moonlight_core::ComparisonRun;
use std::sync::Arc;
use uuid::Uuid;

pub async fn get_health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

pub async fn get_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<moonlight_core::config::AppConfig>, StatusCode> {
    require_admin(&state, &headers)?;
    Ok(Json(state.config.clone()))
}

#[derive(Debug, serde::Deserialize)]
pub struct RunsQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

pub async fn get_runs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<RunsQuery>,
) -> Result<Json<Vec<moonlight_core::ComparisonRunListItem>>, StatusCode> {
    require_admin(&state, &headers)?;
    refresh_storage(&state).await;
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);
    Ok(Json(state.storage.list_page(limit, offset).await))
}

pub async fn get_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ComparisonRun>, StatusCode> {
    require_admin(&state, &headers)?;
    refresh_storage(&state).await;
    state
        .storage
        .get(id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<moonlight_core::StatsSummary>, StatusCode> {
    require_admin(&state, &headers)?;
    refresh_storage(&state).await;
    Ok(Json(state.storage.stats().await))
}

async fn refresh_storage(state: &AppState) {
    if let Err(error) = state.storage.refresh().await {
        eprintln!("failed to refresh moonlight run storage: {error}");
    }
}

fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    let Some(token) = state.config.admin_token.as_deref() else {
        return Ok(());
    };

    if bearer_token_matches(headers, token) || header_token_matches(headers, token) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn bearer_token_matches(headers: &HeaderMap, token: &str) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|value| value == token)
}

fn header_token_matches(headers: &HeaderMap, token: &str) -> bool {
    headers
        .get("x-moonlight-admin-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == token)
}
