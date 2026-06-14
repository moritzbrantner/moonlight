use super::test_support::{
    fetch_runs, spawn_proxy, spawn_target, spawn_target_with_delay,
    spawn_target_with_status_and_delay, test_config, wait_for_run,
};
use axum::http::StatusCode;
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
