use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use moonlight_core::{
    report::{render_report, ReportFormat},
    review::{ReviewUpdate, RunReviewState},
    Adapter, Classification, ComparisonRun, MetricsSnapshot, RunFilter, RunPage,
};
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
    classification: Option<String>,
    adapter: Option<String>,
    q: Option<String>,
    status: Option<u16>,
    has_noise: Option<bool>,
    has_diff: Option<bool>,
}

pub async fn get_runs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<RunsQuery>,
) -> Result<Json<RunPage>, StatusCode> {
    require_admin(&state, &headers)?;
    refresh_storage(&state).await;
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);
    let filter = query.into_filter().map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(
        state.storage.filtered_page(&filter, limit, offset).await,
    ))
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

pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MetricsSnapshot>, StatusCode> {
    require_admin(&state, &headers)?;
    Ok(Json(state.metrics.snapshot()))
}

#[derive(Debug, serde::Deserialize)]
pub struct ReportQuery {
    format: Option<String>,
}

pub async fn get_run_report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<ReportQuery>,
) -> Result<Response, StatusCode> {
    require_admin(&state, &headers)?;
    refresh_storage(&state).await;
    let run = state.storage.get(id).await.ok_or(StatusCode::NOT_FOUND)?;
    let format = query
        .format
        .as_deref()
        .unwrap_or("markdown")
        .parse::<ReportFormat>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let content = render_report(&run, Some(&state.config), format)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let content_type = match format {
        ReportFormat::Markdown => "text/markdown; charset=utf-8",
        ReportFormat::Json => "application/json; charset=utf-8",
    };
    Ok(([(header::CONTENT_TYPE, content_type)], content).into_response())
}

pub async fn get_run_review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<RunReviewState>, StatusCode> {
    require_admin(&state, &headers)?;
    Ok(Json(state.review_store.get(id).await))
}

pub async fn put_run_review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(update): Json<ReviewUpdate>,
) -> Result<Json<RunReviewState>, StatusCode> {
    require_admin(&state, &headers)?;
    state
        .review_store
        .put(id, update)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn refresh_storage(state: &AppState) {
    if let Err(error) = state.storage.refresh().await {
        state.metrics.record_storage_refresh_failure();
        tracing::warn!(error = %error, "failed to refresh moonlight run storage");
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

impl RunsQuery {
    fn into_filter(self) -> anyhow::Result<RunFilter> {
        Ok(RunFilter {
            classification: self
                .classification
                .map(|value| value.parse::<Classification>())
                .transpose()?,
            adapter: self
                .adapter
                .map(|value| value.parse::<Adapter>())
                .transpose()?,
            query: self.q,
            status: self.status,
            has_noise: self.has_noise,
            has_diff: self.has_diff,
        })
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
