use super::{
    target::{
        forward_target, join_optional_target, join_required_target, optional_forward_target,
        response_from_target, selected_response,
    },
    RunMetadata, TargetRequest,
};
use crate::AppState;
use axum::{
    extract::{OriginalUri, State},
    http::Method,
    response::Response,
};
use bytes::Bytes;
use chrono::Utc;
use futures::future::{join3, BoxFuture};
use moonlight_core::{
    compare::{capture_body, capture_headers, compare_targets, CapturedTarget, CompareConfig},
    config::{ResponseTiming, ReturnFallback, ReturnTarget},
    Adapter, ComparisonRun, RunInput,
};
use std::sync::Arc;
use tokio::task::JoinHandle;
use uuid::Uuid;

pub async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    method: Method,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let id = Uuid::new_v4();
    let timestamp = Utc::now();
    let path = uri.path().to_string();
    let query = uri.query().map(ToString::to_string);
    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| path.clone());

    let metadata = RunMetadata {
        id,
        timestamp,
        method: method.to_string(),
        path,
        query,
        request_headers: capture_headers(&headers, &state.config.redact_headers),
        request_body: capture_body(&body, state.config.max_body_capture_bytes),
    };
    let target_request = TargetRequest {
        method,
        path_and_query,
        headers,
        body,
    };

    let primary = forward_target(
        state.clone(),
        "primary",
        state.config.primary_url.clone(),
        target_request.clone(),
    );
    let candidate = forward_target(
        state.clone(),
        "candidate",
        state.config.candidate_url.clone(),
        target_request.clone(),
    );
    let secondary = optional_forward_target(
        state.clone(),
        "secondary",
        state.config.secondary_url.clone(),
        state.config.enable_secondary,
        target_request,
    );

    match state.config.response_timing {
        ResponseTiming::WaitAll => {
            proxy_wait_all(state, metadata, primary, candidate, secondary).await
        }
        ResponseTiming::ReturnSelected => {
            proxy_return_selected(state, metadata, primary, candidate, secondary).await
        }
    }
}

async fn proxy_wait_all(
    state: Arc<AppState>,
    metadata: RunMetadata,
    primary: BoxFuture<'static, CapturedTarget>,
    candidate: BoxFuture<'static, CapturedTarget>,
    secondary: BoxFuture<'static, Option<CapturedTarget>>,
) -> Response {
    let (primary, candidate, secondary) = join3(primary, candidate, secondary).await;
    let response = selected_response(&state, &primary, &candidate);
    let id = metadata.id;
    let run = build_run(
        metadata,
        &primary,
        &candidate,
        secondary.as_ref(),
        &state.config.ignored_json_paths,
        &state.config.ignored_headers,
        state.config.ignore_stderr,
    );

    if let Err(error) = state.storage.insert(run).await {
        eprintln!("failed to persist moonlight comparison run {id}: {error}");
    }

    response
}

async fn proxy_return_selected(
    state: Arc<AppState>,
    metadata: RunMetadata,
    primary: BoxFuture<'static, CapturedTarget>,
    candidate: BoxFuture<'static, CapturedTarget>,
    secondary: BoxFuture<'static, Option<CapturedTarget>>,
) -> Response {
    match state.config.return_target {
        ReturnTarget::Primary => {
            let candidate = tokio::spawn(candidate);
            let secondary = tokio::spawn(secondary);
            let primary = primary.await;
            let response = response_from_target(&primary);
            spawn_persist_run(state, metadata, primary, candidate, secondary);
            response
        }
        ReturnTarget::Candidate => {
            let primary = tokio::spawn(primary);
            let secondary = tokio::spawn(secondary);
            let candidate = candidate.await;
            if candidate.observation.error.is_some()
                && state.config.return_fallback == ReturnFallback::Primary
            {
                let primary = join_required_target(primary, "primary").await;
                let response = response_from_target(&primary);
                spawn_persist_run_with_primary(state, metadata, primary, candidate, secondary);
                response
            } else {
                let response = response_from_target(&candidate);
                spawn_persist_run_with_candidate(state, metadata, primary, candidate, secondary);
                response
            }
        }
    }
}

fn spawn_persist_run(
    state: Arc<AppState>,
    metadata: RunMetadata,
    primary: CapturedTarget,
    candidate: JoinHandle<CapturedTarget>,
    secondary: JoinHandle<Option<CapturedTarget>>,
) {
    tokio::spawn(async move {
        let candidate = join_required_target(candidate, "candidate").await;
        persist_run_with_targets(state, metadata, primary, candidate, secondary).await;
    });
}

fn spawn_persist_run_with_primary(
    state: Arc<AppState>,
    metadata: RunMetadata,
    primary: CapturedTarget,
    candidate: CapturedTarget,
    secondary: JoinHandle<Option<CapturedTarget>>,
) {
    tokio::spawn(async move {
        persist_run_with_targets(state, metadata, primary, candidate, secondary).await;
    });
}

fn spawn_persist_run_with_candidate(
    state: Arc<AppState>,
    metadata: RunMetadata,
    primary: JoinHandle<CapturedTarget>,
    candidate: CapturedTarget,
    secondary: JoinHandle<Option<CapturedTarget>>,
) {
    tokio::spawn(async move {
        let primary = join_required_target(primary, "primary").await;
        persist_run_with_targets(state, metadata, primary, candidate, secondary).await;
    });
}

async fn persist_run_with_targets(
    state: Arc<AppState>,
    metadata: RunMetadata,
    primary: CapturedTarget,
    candidate: CapturedTarget,
    secondary: JoinHandle<Option<CapturedTarget>>,
) {
    let secondary = join_optional_target(secondary, "secondary").await;
    let id = metadata.id;
    let run = build_run(
        metadata,
        &primary,
        &candidate,
        secondary.as_ref(),
        &state.config.ignored_json_paths,
        &state.config.ignored_headers,
        state.config.ignore_stderr,
    );

    if let Err(error) = state.storage.insert(run).await {
        eprintln!("failed to persist moonlight comparison run {id}: {error}");
    }
}

fn build_run(
    metadata: RunMetadata,
    primary: &CapturedTarget,
    candidate: &CapturedTarget,
    secondary: Option<&CapturedTarget>,
    ignored_json_paths: &[String],
    ignored_headers: &[String],
    ignore_stderr: bool,
) -> ComparisonRun {
    let compare_config = CompareConfig::new(ignored_json_paths, ignored_headers, ignore_stderr);
    let comparison = compare_targets(primary, candidate, secondary, &compare_config);

    ComparisonRun {
        id: metadata.id,
        timestamp: metadata.timestamp,
        adapter: Adapter::Http,
        input: RunInput::Http {
            method: metadata.method,
            path: metadata.path,
            query: metadata.query,
        },
        request_headers: metadata.request_headers,
        request_body: metadata.request_body,
        primary: primary.observation.clone(),
        candidate: candidate.observation.clone(),
        secondary: secondary.map(|target| target.observation.clone()),
        comparison,
    }
}
