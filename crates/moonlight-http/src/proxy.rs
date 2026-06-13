use crate::AppState;
use axum::{
    body::Body,
    extract::{OriginalUri, Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use chrono::Utc;
use futures::future::{join3, BoxFuture};
use moonlight_core::{
    compare::{
        capture_body, capture_headers, compare_targets, is_hop_by_hop_header, CapturedTarget,
        CompareConfig,
    },
    config::{ResponseTiming, ReturnFallback, ReturnTarget},
    Adapter, BodyCapture, ComparisonRun, RunInput, TargetObservation,
};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
use tokio::task::JoinHandle;
use uuid::Uuid;

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

pub async fn get_health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

pub async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.config.clone())
}

pub async fn get_runs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if let Err(error) = state.storage.refresh().await {
        eprintln!("failed to refresh moonlight run storage: {error}");
    }
    Json(state.storage.list().await)
}

pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ComparisonRun>, StatusCode> {
    if let Err(error) = state.storage.refresh().await {
        eprintln!("failed to refresh moonlight run storage: {error}");
    }
    state
        .storage
        .get(id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if let Err(error) = state.storage.refresh().await {
        eprintln!("failed to refresh moonlight run storage: {error}");
    }
    Json(state.storage.stats().await)
}

pub async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
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
        persist_run(state, metadata, primary, candidate, secondary).await;
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

async fn persist_run(
    state: Arc<AppState>,
    metadata: RunMetadata,
    primary: CapturedTarget,
    candidate: CapturedTarget,
    secondary: JoinHandle<Option<CapturedTarget>>,
) {
    persist_run_with_targets(state, metadata, primary, candidate, secondary).await;
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

async fn join_required_target(
    handle: JoinHandle<CapturedTarget>,
    label: &'static str,
) -> CapturedTarget {
    match handle.await {
        Ok(target) => target,
        Err(error) => task_error_target(label, format!("{label} task failed: {error}")),
    }
}

async fn join_optional_target(
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

fn selected_response(
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

fn optional_forward_target(
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

fn forward_target(
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

        match target_request.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let headers = capture_headers(response.headers(), &state.config.redact_headers);
                match response.bytes().await {
                    Ok(body_bytes) => CapturedTarget {
                        observation: TargetObservation {
                            status: Some(status),
                            headers,
                            body: capture_body(&body_bytes, state.config.max_body_capture_bytes),
                            stderr: None,
                            latency_ms: started.elapsed().as_millis(),
                            error: None,
                        },
                        body_bytes,
                        stderr_bytes: Bytes::new(),
                    },
                    Err(error) => error_target(
                        started,
                        Some(status),
                        headers,
                        format!("{label} body read failed: {error}"),
                    ),
                }
            }
            Err(error) => error_target(
                started,
                None,
                Default::default(),
                format!("{label} request failed: {error}"),
            ),
        }
    })
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

fn task_error_target(label: &'static str, error: String) -> CapturedTarget {
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

fn response_from_target(target: &CapturedTarget) -> Response {
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

#[allow(dead_code)]
fn _uri_path(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| "/".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_router, build_state};
    use axum::{routing::any, Router};
    use moonlight_core::{
        config::{AppConfig, ResponseTiming, ReturnFallback, ReturnTarget},
        Classification, ComparisonRunListItem, RunInput,
    };
    use std::{
        net::SocketAddr,
        path::PathBuf,
        time::{Duration, Instant},
    };
    use tempfile::tempdir;
    use tokio::net::TcpListener;

    async fn spawn_target(body: &'static str) -> SocketAddr {
        spawn_target_with_status_and_delay(StatusCode::OK, body, Duration::ZERO).await
    }

    async fn spawn_target_with_delay(body: &'static str, delay: Duration) -> SocketAddr {
        spawn_target_with_status_and_delay(StatusCode::OK, body, delay).await
    }

    async fn spawn_target_with_status_and_delay(
        status: StatusCode,
        body: &'static str,
        delay: Duration,
    ) -> SocketAddr {
        async fn handler(
            State((status, body, delay)): State<(StatusCode, &'static str, Duration)>,
        ) -> impl IntoResponse {
            tokio::time::sleep(delay).await;
            (status, [("content-type", "application/json")], body)
        }

        let app = Router::new()
            .fallback(any(handler))
            .with_state((status, body, delay));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        addr
    }

    fn test_config(
        primary: SocketAddr,
        candidate: SocketAddr,
        dir: &tempfile::TempDir,
        response_timing: ResponseTiming,
    ) -> AppConfig {
        AppConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            primary_url: format!("http://{primary}"),
            candidate_url: format!("http://{candidate}"),
            secondary_url: "http://127.0.0.1:9".to_string(),
            enable_secondary: false,
            return_target: ReturnTarget::Primary,
            return_fallback: ReturnFallback::None,
            response_timing,
            max_body_capture_bytes: 1024,
            redact_headers: vec![
                "authorization".into(),
                "cookie".into(),
                "set-cookie".into(),
                "x-api-key".into(),
            ],
            ignored_json_paths: vec![
                "$.timestamp".into(),
                "$.requestId".into(),
                "$.traceId".into(),
                "$.id".into(),
            ],
            ignored_headers: vec![
                "date".into(),
                "server".into(),
                "set-cookie".into(),
                "x-request-id".into(),
                "traceparent".into(),
            ],
            ignore_stderr: false,
            storage_path: PathBuf::from(dir.path()).join("http-runs.jsonl"),
        }
    }

    async fn spawn_proxy(config: AppConfig) -> SocketAddr {
        let state = build_state(config).await.unwrap();
        let app = build_router(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        addr
    }

    async fn fetch_runs(
        client: &reqwest::Client,
        proxy_addr: SocketAddr,
    ) -> Vec<ComparisonRunListItem> {
        client
            .get(format!("http://{proxy_addr}/api/runs"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    async fn wait_for_run(
        client: &reqwest::Client,
        proxy_addr: SocketAddr,
    ) -> ComparisonRunListItem {
        for _ in 0..40 {
            let runs = fetch_runs(client, proxy_addr).await;
            if let Some(run) = runs.into_iter().next() {
                return run;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        panic!("comparison run was not stored");
    }

    #[tokio::test]
    async fn proxy_returns_primary_and_records_run() {
        let primary = spawn_target(r#"{"source":"primary"}"#).await;
        let candidate = spawn_target(r#"{"source":"candidate"}"#).await;
        let dir = tempdir().unwrap();
        let config = test_config(primary, candidate, &dir, ResponseTiming::WaitAll);
        let proxy_addr = spawn_proxy(config).await;

        let client = reqwest::Client::new();
        let body = client
            .get(format!("http://{proxy_addr}/anything?x=1"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(body, r#"{"source":"primary"}"#);

        let runs = fetch_runs(&client, proxy_addr).await;
        assert_eq!(runs.len(), 1);
        assert!(matches!(
            runs[0].input,
            RunInput::Http { ref path, .. } if path == "/anything"
        ));
        assert_eq!(runs[0].classification, Classification::SuspiciousDifference);
    }

    #[tokio::test]
    async fn wait_all_response_timing_waits_for_slow_candidate() {
        let primary = spawn_target(r#"{"source":"primary"}"#).await;
        let candidate =
            spawn_target_with_delay(r#"{"source":"candidate"}"#, Duration::from_millis(175)).await;
        let dir = tempdir().unwrap();
        let config = test_config(primary, candidate, &dir, ResponseTiming::WaitAll);
        let proxy_addr = spawn_proxy(config).await;
        let client = reqwest::Client::new();

        let started = Instant::now();
        let body = client
            .get(format!("http://{proxy_addr}/anything"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        assert_eq!(body, r#"{"source":"primary"}"#);
        assert!(started.elapsed() >= Duration::from_millis(150));
    }

    #[tokio::test]
    async fn return_selected_response_timing_returns_before_slow_candidate_and_records_later() {
        let primary = spawn_target(r#"{"source":"primary"}"#).await;
        let candidate =
            spawn_target_with_delay(r#"{"source":"candidate"}"#, Duration::from_millis(300)).await;
        let dir = tempdir().unwrap();
        let config = test_config(primary, candidate, &dir, ResponseTiming::ReturnSelected);
        let proxy_addr = spawn_proxy(config).await;
        let client = reqwest::Client::new();

        let started = Instant::now();
        let body = client
            .get(format!("http://{proxy_addr}/anything"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        assert_eq!(body, r#"{"source":"primary"}"#);
        assert!(started.elapsed() < Duration::from_millis(250));

        let run = wait_for_run(&client, proxy_addr).await;
        assert_eq!(run.classification, Classification::SuspiciousDifference);
    }

    #[tokio::test]
    async fn can_return_candidate_response() {
        let primary = spawn_target(r#"{"source":"primary"}"#).await;
        let candidate = spawn_target(r#"{"source":"candidate"}"#).await;
        let dir = tempdir().unwrap();
        let mut config = test_config(primary, candidate, &dir, ResponseTiming::WaitAll);
        config.return_target = ReturnTarget::Candidate;
        let proxy_addr = spawn_proxy(config).await;
        let client = reqwest::Client::new();

        let body = client
            .get(format!("http://{proxy_addr}/anything"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        assert_eq!(body, r#"{"source":"candidate"}"#);
    }

    #[tokio::test]
    async fn candidate_return_can_fallback_to_primary() {
        let primary = spawn_target(r#"{"source":"primary"}"#).await;
        let candidate = spawn_target_with_status_and_delay(
            StatusCode::INTERNAL_SERVER_ERROR,
            "fail",
            Duration::ZERO,
        )
        .await;
        let dir = tempdir().unwrap();
        let mut config = test_config(primary, candidate, &dir, ResponseTiming::WaitAll);
        config.return_target = ReturnTarget::Candidate;
        config.return_fallback = ReturnFallback::Primary;
        let proxy_addr = spawn_proxy(config).await;
        let client = reqwest::Client::new();

        let body = client
            .get(format!("http://{proxy_addr}/anything"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        assert_eq!(body, "fail");
    }

    #[tokio::test]
    async fn candidate_transport_error_falls_back_to_primary_when_configured() {
        let primary = spawn_target(r#"{"source":"primary"}"#).await;
        let dir = tempdir().unwrap();
        let mut config = test_config(
            primary,
            "127.0.0.1:9".parse().unwrap(),
            &dir,
            ResponseTiming::WaitAll,
        );
        config.return_target = ReturnTarget::Candidate;
        config.return_fallback = ReturnFallback::Primary;
        let proxy_addr = spawn_proxy(config).await;
        let client = reqwest::Client::new();

        let body = client
            .get(format!("http://{proxy_addr}/anything"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        assert_eq!(body, r#"{"source":"primary"}"#);
    }

    #[tokio::test]
    async fn target_error_is_recorded_when_candidate_transport_fails() {
        let primary = spawn_target(r#"{"source":"primary"}"#).await;
        let dir = tempdir().unwrap();
        let config = test_config(
            primary,
            "127.0.0.1:9".parse().unwrap(),
            &dir,
            ResponseTiming::WaitAll,
        );
        let proxy_addr = spawn_proxy(config).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{proxy_addr}/anything"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let run = wait_for_run(&client, proxy_addr).await;
        assert_eq!(run.classification, Classification::TargetError);
    }
}
