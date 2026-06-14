use crate::{build_router, build_state};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::any, Router};
use moonlight_core::{
    config::{AppConfig, ResponseTiming, ReturnFallback, ReturnTarget},
    ComparisonRunListItem,
};
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use tokio::net::TcpListener;

pub(super) async fn spawn_target(body: &'static str) -> SocketAddr {
    spawn_target_with_status_and_delay(StatusCode::OK, body, Duration::ZERO).await
}

pub(super) async fn spawn_target_with_delay(body: &'static str, delay: Duration) -> SocketAddr {
    spawn_target_with_status_and_delay(StatusCode::OK, body, delay).await
}

pub(super) async fn spawn_target_with_status_and_delay(
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

pub(super) fn test_config(
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

pub(super) async fn spawn_proxy(config: AppConfig) -> SocketAddr {
    let state = build_state(config).await.unwrap();
    let app = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

pub(super) async fn fetch_runs(
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

pub(super) async fn wait_for_run(
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
