use super::test_support::{
    fetch_runs, spawn_proxy, spawn_target, spawn_target_with_delay,
    spawn_target_with_status_and_delay, spawn_uri_target, test_config, wait_for_run,
};
use axum::http::{header, HeaderValue, StatusCode};
use moonlight_core::{
    config::{ResponseTiming, ReturnFallback, ReturnTarget},
    Classification, RunInput,
};
use std::time::{Duration, Instant};
use tempfile::tempdir;

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

#[tokio::test]
async fn admin_token_protects_admin_routes_but_not_health_or_proxy() {
    let primary = spawn_target(r#"{"source":"primary"}"#).await;
    let candidate = spawn_target(r#"{"source":"candidate"}"#).await;
    let dir = tempdir().unwrap();
    let mut config = test_config(primary, candidate, &dir, ResponseTiming::WaitAll);
    config.admin_token = Some("secret".to_string());
    let proxy_addr = spawn_proxy(config).await;
    let client = reqwest::Client::new();

    let unauthorized = client
        .get(format!("http://{proxy_addr}/api/runs"))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let health = client
        .get(format!("http://{proxy_addr}/api/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);

    let proxy_body = client
        .get(format!("http://{proxy_addr}/anything"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(proxy_body, r#"{"source":"primary"}"#);

    let authorized = client
        .get(format!("http://{proxy_addr}/api/runs"))
        .bearer_auth("secret")
        .send()
        .await
        .unwrap();
    assert_eq!(authorized.status(), StatusCode::OK);
}

#[tokio::test]
async fn cors_allows_configured_origin_and_rejects_unlisted_origin() {
    let primary = spawn_target(r#"{"source":"primary"}"#).await;
    let candidate = spawn_target(r#"{"source":"candidate"}"#).await;
    let dir = tempdir().unwrap();
    let config = test_config(primary, candidate, &dir, ResponseTiming::WaitAll);
    let proxy_addr = spawn_proxy(config).await;
    let client = reqwest::Client::new();

    let allowed = client
        .get(format!("http://{proxy_addr}/api/health"))
        .header(header::ORIGIN, "http://127.0.0.1:5173")
        .send()
        .await
        .unwrap();
    assert_eq!(
        allowed.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&HeaderValue::from_static("http://127.0.0.1:5173"))
    );

    let rejected = client
        .get(format!("http://{proxy_addr}/api/health"))
        .header(header::ORIGIN, "http://evil.test")
        .send()
        .await
        .unwrap();
    assert!(rejected
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}

#[tokio::test]
async fn oversized_request_returns_payload_too_large() {
    let primary = spawn_target(r#"{"source":"primary"}"#).await;
    let candidate = spawn_target(r#"{"source":"candidate"}"#).await;
    let dir = tempdir().unwrap();
    let mut config = test_config(primary, candidate, &dir, ResponseTiming::WaitAll);
    config.max_request_body_bytes = 3;
    let proxy_addr = spawn_proxy(config).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://{proxy_addr}/anything"))
        .body("too large")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn query_param_redaction_stores_redacted_query_but_forwards_original() {
    let primary = spawn_uri_target().await;
    let candidate = spawn_uri_target().await;
    let dir = tempdir().unwrap();
    let config = test_config(primary, candidate, &dir, ResponseTiming::WaitAll);
    let proxy_addr = spawn_proxy(config).await;
    let client = reqwest::Client::new();

    let body = client
        .get(format!(
            "http://{proxy_addr}/anything?token=secret&safe=visible"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(body, "/anything?token=secret&safe=visible");
    let runs = fetch_runs(&client, proxy_addr).await;
    assert!(matches!(
        runs[0].input,
        RunInput::Http { ref query, .. }
            if query.as_deref() == Some("token=[redacted]&safe=visible")
    ));
}
