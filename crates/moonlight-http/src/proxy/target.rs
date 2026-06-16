use super::TargetRequest;
use crate::AppState;
use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use futures::future::BoxFuture;
use moonlight_core::{
    compare::{
        capture_body, capture_body_with_redactions, capture_headers, is_hop_by_hop_header,
        CapturedTarget,
    },
    config::{ReturnFallback, ReturnTarget},
    TargetObservation,
};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
use tokio::{
    task::JoinHandle,
    time::{timeout, Duration},
};

pub(super) fn optional_forward_target(
    state: Arc<AppState>,
    label: &'static str,
    base_url: String,
    enabled: bool,
    request: TargetRequest,
) -> BoxFuture<'static, Option<CapturedTarget>> {
    Box::pin(async move {
        if enabled {
            Some(forward_target(state, label, base_url, request).await)
        } else {
            None
        }
    })
}

pub(super) fn forward_target(
    state: Arc<AppState>,
    label: &'static str,
    base_url: String,
    request: TargetRequest,
) -> BoxFuture<'static, CapturedTarget> {
    Box::pin(async move {
        let started = Instant::now();
        let url = format!("{base_url}{}", request.path_and_query);
        let reqwest_method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .unwrap_or(reqwest::Method::GET);
        let mut target_request = state
            .client
            .request(reqwest_method, url)
            .body(request.body.clone());
        for (name, value) in &request.headers {
            if is_hop_by_hop_header(name.as_str()) {
                continue;
            }
            target_request = target_request.header(name.as_str(), value.as_bytes());
        }

        match timeout(
            Duration::from_millis(state.config.target_timeout_ms),
            target_request.send(),
        )
        .await
        {
            Ok(Ok(response)) => capture_response(state, label, started, response).await,
            Ok(Err(error)) => error_target(
                started,
                None,
                Default::default(),
                format!("{label} request failed: {error}"),
            ),
            Err(_) => error_target(
                started,
                None,
                Default::default(),
                format!(
                    "{label} request timed out after {} ms",
                    state.config.target_timeout_ms
                ),
            ),
        }
    })
}

pub(super) async fn join_required_target(
    handle: JoinHandle<CapturedTarget>,
    label: &'static str,
) -> CapturedTarget {
    match handle.await {
        Ok(target) => target,
        Err(error) => task_error_target(label, format!("{label} task failed: {error}")),
    }
}

pub(super) async fn join_optional_target(
    handle: JoinHandle<Option<CapturedTarget>>,
    label: &'static str,
) -> Option<CapturedTarget> {
    match handle.await {
        Ok(target) => target,
        Err(error) => Some(task_error_target(
            label,
            format!("{label} task failed: {error}"),
        )),
    }
}

pub(super) fn selected_response(
    state: &AppState,
    primary: &CapturedTarget,
    candidate: &CapturedTarget,
) -> Response {
    match state.config.return_target {
        ReturnTarget::Primary => response_from_target(primary),
        ReturnTarget::Candidate => {
            if candidate.observation.error.is_some()
                && state.config.return_fallback == ReturnFallback::Primary
            {
                response_from_target(primary)
            } else {
                response_from_target(candidate)
            }
        }
    }
}

pub(super) fn response_from_target(target: &CapturedTarget) -> Response {
    if let Some(error) = &target.observation.error {
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response();
    }

    let status = target
        .observation
        .status
        .and_then(|status| StatusCode::from_u16(status).ok())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);
    for (name, value) in &target.observation.headers {
        if is_hop_by_hop_header(name) {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder
        .body(Body::from(target.body_bytes.clone()))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

pub(super) fn task_error_target(label: &'static str, error: String) -> CapturedTarget {
    CapturedTarget {
        observation: TargetObservation {
            status: None,
            headers: BTreeMap::new(),
            body: capture_body(&[], 0),
            stderr: None,
            latency_ms: 0,
            error: Some(format!("{label} target task failed: {error}")),
        },
        body_bytes: Bytes::new(),
        stderr_bytes: Bytes::new(),
    }
}

async fn capture_response(
    state: Arc<AppState>,
    label: &'static str,
    started: Instant,
    response: reqwest::Response,
) -> CapturedTarget {
    let status = response.status().as_u16();
    let headers = capture_headers(response.headers(), &state.config.redact_headers);
    match timeout(
        Duration::from_millis(state.config.target_timeout_ms),
        response.bytes(),
    )
    .await
    {
        Ok(Ok(body_bytes)) => CapturedTarget {
            observation: TargetObservation {
                status: Some(status),
                headers,
                body: capture_body_with_redactions(
                    &body_bytes,
                    state.config.max_body_capture_bytes,
                    &state.config.redact_json_paths,
                ),
                stderr: None,
                latency_ms: started.elapsed().as_millis(),
                error: None,
            },
            body_bytes,
            stderr_bytes: Bytes::new(),
        },
        Ok(Err(error)) => error_target(
            started,
            Some(status),
            headers,
            format!("{label} body read failed: {error}"),
        ),
        Err(_) => error_target(
            started,
            Some(status),
            headers,
            format!(
                "{label} body read timed out after {} ms",
                state.config.target_timeout_ms
            ),
        ),
    }
}

fn error_target(
    started: Instant,
    status: Option<u16>,
    headers: BTreeMap<String, String>,
    error: String,
) -> CapturedTarget {
    CapturedTarget {
        observation: TargetObservation {
            status,
            headers,
            body: capture_body(&[], 0),
            stderr: None,
            latency_ms: started.elapsed().as_millis(),
            error: Some(error),
        },
        body_bytes: Bytes::new(),
        stderr_bytes: Bytes::new(),
    }
}
