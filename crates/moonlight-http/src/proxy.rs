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
        capture_body, capture_headers, compare_backends, is_hop_by_hop_header, CapturedBackend,
        CompareConfig,
    },
    config::ResponseMode,
    BackendCapture, BodyCapture, RequestRecord,
};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
use tokio::task::JoinHandle;
use uuid::Uuid;

struct RequestMetadata {
    id: Uuid,
    timestamp: chrono::DateTime<Utc>,
    method: String,
    path: String,
    query: Option<String>,
    request_headers: BTreeMap<String, String>,
    request_body: BodyCapture,
}

pub async fn get_health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

pub async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.config.clone())
}

pub async fn get_requests(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.storage.list().await)
}

pub async fn get_request(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<RequestRecord>, StatusCode> {
    state
        .storage
        .get(id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

    let metadata = RequestMetadata {
        id,
        timestamp,
        method: method.to_string(),
        path,
        query,
        request_headers: capture_headers(&headers, &state.config.redact_headers),
        request_body: capture_body(&body, state.config.max_body_capture_bytes),
    };

    let primary = forward_backend(
        state.clone(),
        "primary",
        state.config.primary_url.clone(),
        method.clone(),
        path_and_query.clone(),
        headers.clone(),
        body.clone(),
    );

    let candidate = optional_forward(
        state.clone(),
        "candidate",
        state.config.candidate_url.clone(),
        state.config.enable_candidate,
        method.clone(),
        path_and_query.clone(),
        headers.clone(),
        body.clone(),
    );

    let secondary = optional_forward(
        state.clone(),
        "secondary",
        state.config.secondary_url.clone(),
        state.config.enable_secondary,
        method.clone(),
        path_and_query,
        headers.clone(),
        body.clone(),
    );

    match state.config.response_mode {
        ResponseMode::WaitAll => {
            proxy_wait_all(state, metadata, primary, candidate, secondary).await
        }
        ResponseMode::PrimaryThenShadow => {
            proxy_primary_then_shadow(state, metadata, primary, candidate, secondary).await
        }
    }
}

async fn proxy_wait_all(
    state: Arc<AppState>,
    metadata: RequestMetadata,
    primary: BoxFuture<'static, CapturedBackend>,
    candidate: BoxFuture<'static, Option<CapturedBackend>>,
    secondary: BoxFuture<'static, Option<CapturedBackend>>,
) -> Response {
    let (primary, candidate, secondary) = join3(primary, candidate, secondary).await;
    let id = metadata.id;
    let record = build_record(
        metadata,
        &primary,
        candidate.as_ref(),
        secondary.as_ref(),
        &state.config.ignored_json_paths,
        &state.config.ignored_headers,
    );

    if let Err(error) = state.storage.insert(record).await {
        eprintln!("failed to persist shadowdiff request record {id}: {error}");
    }

    response_from_primary(&primary)
}

async fn proxy_primary_then_shadow(
    state: Arc<AppState>,
    metadata: RequestMetadata,
    primary: BoxFuture<'static, CapturedBackend>,
    candidate: BoxFuture<'static, Option<CapturedBackend>>,
    secondary: BoxFuture<'static, Option<CapturedBackend>>,
) -> Response {
    let candidate = tokio::spawn(candidate);
    let secondary = tokio::spawn(secondary);
    let primary = primary.await;
    let response = response_from_primary(&primary);
    let background_state = state.clone();

    tokio::spawn(async move {
        let candidate = join_optional_backend(candidate, "candidate").await;
        let secondary = join_optional_backend(secondary, "secondary").await;
        let record = build_record(
            metadata,
            &primary,
            candidate.as_ref(),
            secondary.as_ref(),
            &background_state.config.ignored_json_paths,
            &background_state.config.ignored_headers,
        );
        let id = record.id;

        if let Err(error) = background_state.storage.insert(record).await {
            eprintln!("failed to persist shadowdiff request record {id}: {error}");
        }
    });

    response
}

async fn join_optional_backend(
    handle: JoinHandle<Option<CapturedBackend>>,
    label: &'static str,
) -> Option<CapturedBackend> {
    match handle.await {
        Ok(backend) => backend,
        Err(error) => {
            eprintln!("shadowdiff {label} task failed: {error}");
            None
        }
    }
}

fn build_record(
    metadata: RequestMetadata,
    primary: &CapturedBackend,
    candidate: Option<&CapturedBackend>,
    secondary: Option<&CapturedBackend>,
    ignored_json_paths: &[String],
    ignored_headers: &[String],
) -> RequestRecord {
    let compare_config = CompareConfig::new(ignored_json_paths, ignored_headers);
    let comparison = compare_backends(primary, candidate, secondary, &compare_config);

    RequestRecord {
        id: metadata.id,
        timestamp: metadata.timestamp,
        method: metadata.method,
        path: metadata.path,
        query: metadata.query,
        request_headers: metadata.request_headers,
        request_body: metadata.request_body,
        primary: primary.capture.clone(),
        candidate: candidate.map(|backend| backend.capture.clone()),
        secondary: secondary.map(|backend| backend.capture.clone()),
        comparison,
    }
}

fn optional_forward(
    state: Arc<AppState>,
    label: &'static str,
    base_url: String,
    enabled: bool,
    method: Method,
    path_and_query: String,
    headers: HeaderMap,
    body: Bytes,
) -> BoxFuture<'static, Option<CapturedBackend>> {
    Box::pin(async move {
        if enabled {
            Some(
                forward_backend(
                    state,
                    label,
                    base_url,
                    method,
                    path_and_query,
                    headers,
                    body,
                )
                .await,
            )
        } else {
            None
        }
    })
}

fn forward_backend(
    state: Arc<AppState>,
    label: &'static str,
    base_url: String,
    method: Method,
    path_and_query: String,
    headers: HeaderMap,
    body: Bytes,
) -> BoxFuture<'static, CapturedBackend> {
    Box::pin(async move {
        let started = Instant::now();
        let url = format!("{base_url}{path_and_query}");
        let reqwest_method =
            reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET);
        let mut request = state.client.request(reqwest_method, url).body(body.clone());
        for (name, value) in &headers {
            if is_hop_by_hop_header(name.as_str()) {
                continue;
            }
            request = request.header(name.as_str(), value.as_bytes());
        }

        match request.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let headers = capture_headers(response.headers(), &state.config.redact_headers);
                match response.bytes().await {
                    Ok(body_bytes) => CapturedBackend {
                        capture: BackendCapture {
                            status: Some(status),
                            headers,
                            body: capture_body(&body_bytes, state.config.max_body_capture_bytes),
                            latency_ms: started.elapsed().as_millis(),
                            error: None,
                        },
                        body_bytes,
                    },
                    Err(error) => error_backend(
                        started,
                        Some(status),
                        headers,
                        format!("{label} body read failed: {error}"),
                    ),
                }
            }
            Err(error) => error_backend(
                started,
                None,
                Default::default(),
                format!("{label} request failed: {error}"),
            ),
        }
    })
}

fn error_backend(
    started: Instant,
    status: Option<u16>,
    headers: std::collections::BTreeMap<String, String>,
    error: String,
) -> CapturedBackend {
    CapturedBackend {
        capture: BackendCapture {
            status,
            headers,
            body: capture_body(&[], 0),
            latency_ms: started.elapsed().as_millis(),
            error: Some(error),
        },
        body_bytes: Bytes::new(),
    }
}

fn response_from_primary(primary: &CapturedBackend) -> Response {
    if let Some(error) = &primary.capture.error {
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response();
    }

    let status = primary
        .capture
        .status
        .and_then(|status| StatusCode::from_u16(status).ok())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);
    for (name, value) in &primary.capture.headers {
        if is_hop_by_hop_header(name) {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder
        .body(Body::from(primary.body_bytes.clone()))
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
    use moonlight_core::config::{AppConfig, ResponseMode, ReturnBackend};
    use std::{
        net::SocketAddr,
        path::PathBuf,
        time::{Duration, Instant},
    };
    use tempfile::tempdir;
    use tokio::net::TcpListener;

    async fn spawn_backend(body: &'static str) -> SocketAddr {
        spawn_backend_with_delay(body, Duration::ZERO).await
    }

    async fn spawn_backend_with_delay(body: &'static str, delay: Duration) -> SocketAddr {
        async fn handler(
            State((body, delay)): State<(&'static str, Duration)>,
        ) -> impl IntoResponse {
            tokio::time::sleep(delay).await;
            (StatusCode::OK, [("content-type", "application/json")], body)
        }

        let app = Router::new()
            .fallback(any(handler))
            .with_state((body, delay));
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
        response_mode: ResponseMode,
    ) -> AppConfig {
        AppConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            primary_url: format!("http://{primary}"),
            candidate_url: format!("http://{candidate}"),
            secondary_url: "http://127.0.0.1:9".to_string(),
            enable_candidate: true,
            enable_secondary: false,
            return_backend: ReturnBackend::Primary,
            response_mode,
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
            storage_path: PathBuf::from(dir.path()).join("requests.jsonl"),
        }
    }

    async fn spawn_proxy(config: AppConfig) -> SocketAddr {
        let state = build_state(config).await.unwrap();
        let app = build_router(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        proxy_addr
    }

    async fn fetch_requests(
        client: &reqwest::Client,
        proxy_addr: SocketAddr,
    ) -> Vec<moonlight_core::RequestListItem> {
        client
            .get(format!("http://{proxy_addr}/api/requests"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    async fn wait_for_request(
        client: &reqwest::Client,
        proxy_addr: SocketAddr,
    ) -> moonlight_core::RequestListItem {
        for _ in 0..40 {
            let requests = fetch_requests(client, proxy_addr).await;
            if let Some(request) = requests.into_iter().next() {
                return request;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        panic!("request record was not stored");
    }

    #[tokio::test]
    async fn proxy_returns_primary_and_records_request() {
        let primary = spawn_backend(r#"{"source":"primary"}"#).await;
        let candidate = spawn_backend(r#"{"source":"candidate"}"#).await;
        let dir = tempdir().unwrap();
        let config = test_config(primary, candidate, &dir, ResponseMode::WaitAll);
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

        let requests = fetch_requests(&client, proxy_addr).await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, "/anything");
        assert_eq!(
            requests[0].classification,
            moonlight_core::Classification::CandidateDiff
        );
    }

    #[tokio::test]
    async fn wait_all_response_mode_waits_for_slow_candidate() {
        let primary = spawn_backend(r#"{"source":"primary"}"#).await;
        let candidate =
            spawn_backend_with_delay(r#"{"source":"candidate"}"#, Duration::from_millis(175)).await;
        let dir = tempdir().unwrap();
        let config = test_config(primary, candidate, &dir, ResponseMode::WaitAll);
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
    async fn primary_then_shadow_returns_before_slow_candidate_and_records_later() {
        let primary = spawn_backend(r#"{"source":"primary"}"#).await;
        let candidate =
            spawn_backend_with_delay(r#"{"source":"candidate"}"#, Duration::from_millis(300)).await;
        let dir = tempdir().unwrap();
        let config = test_config(primary, candidate, &dir, ResponseMode::PrimaryThenShadow);
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
        assert!(started.elapsed() < Duration::from_millis(150));

        let request = wait_for_request(&client, proxy_addr).await;
        assert_eq!(
            request.classification,
            moonlight_core::Classification::CandidateDiff
        );
    }

    #[tokio::test]
    async fn primary_then_shadow_records_candidate_backend_error_without_failing_primary() {
        let primary = spawn_backend(r#"{"source":"primary"}"#).await;
        let candidate: SocketAddr = "127.0.0.1:9".parse().unwrap();
        let dir = tempdir().unwrap();
        let config = test_config(primary, candidate, &dir, ResponseMode::PrimaryThenShadow);
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

        let request = wait_for_request(&client, proxy_addr).await;
        assert_eq!(
            request.classification,
            moonlight_core::Classification::BackendError
        );
    }
}
