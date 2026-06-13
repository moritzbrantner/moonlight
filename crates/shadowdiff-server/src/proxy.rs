use crate::{
    compare::{
        capture_body, capture_headers, compare_backends, is_hop_by_hop_header, CapturedBackend,
        CompareConfig,
    },
    AppState,
};
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
use shadowdiff_types::{BackendCapture, RequestRecord};
use std::{sync::Arc, time::Instant};
use uuid::Uuid;

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

    let compare_config = CompareConfig::new(
        &state.config.ignored_json_paths,
        &state.config.ignored_headers,
    );
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

    let (primary, candidate, secondary) = join3(primary, candidate, secondary).await;
    let comparison = compare_backends(
        &primary,
        candidate.as_ref(),
        secondary.as_ref(),
        &compare_config,
    );
    let record = RequestRecord {
        id,
        timestamp,
        method: method.to_string(),
        path,
        query,
        request_headers: capture_headers(&headers, &state.config.redact_headers),
        request_body: capture_body(&body, state.config.max_body_capture_bytes),
        primary: primary.capture.clone(),
        candidate: candidate.as_ref().map(|backend| backend.capture.clone()),
        secondary: secondary.as_ref().map(|backend| backend.capture.clone()),
        comparison,
    };

    if let Err(error) = state.storage.insert(record).await {
        eprintln!("failed to persist shadowdiff request record {id}: {error}");
    }

    response_from_primary(primary)
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

fn response_from_primary(primary: CapturedBackend) -> Response {
    if let Some(error) = primary.capture.error {
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
    for (name, value) in primary.capture.headers {
        if is_hop_by_hop_header(&name) {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder
        .body(Body::from(primary.body_bytes))
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
    use crate::{
        build_router, build_state,
        config::{AppConfig, ReturnBackend},
    };
    use axum::{routing::any, Router};
    use std::{net::SocketAddr, path::PathBuf};
    use tempfile::tempdir;
    use tokio::net::TcpListener;

    async fn spawn_backend(body: &'static str) -> SocketAddr {
        async fn handler(State(body): State<&'static str>) -> impl IntoResponse {
            (StatusCode::OK, [("content-type", "application/json")], body)
        }

        let app = Router::new().fallback(any(handler)).with_state(body);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        addr
    }

    #[tokio::test]
    async fn proxy_returns_primary_and_records_request() {
        let primary = spawn_backend(r#"{"source":"primary"}"#).await;
        let candidate = spawn_backend(r#"{"source":"candidate"}"#).await;
        let dir = tempdir().unwrap();
        let config = AppConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            primary_url: format!("http://{primary}"),
            candidate_url: format!("http://{candidate}"),
            secondary_url: "http://127.0.0.1:9".to_string(),
            enable_candidate: true,
            enable_secondary: false,
            return_backend: ReturnBackend::Primary,
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
        };
        let state = build_state(config).await.unwrap();
        let app = build_router(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

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

        let requests: Vec<shadowdiff_types::RequestListItem> = client
            .get(format!("http://{proxy_addr}/api/requests"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, "/anything");
        assert_eq!(
            requests[0].classification,
            shadowdiff_types::Classification::CandidateDiff
        );
    }
}
