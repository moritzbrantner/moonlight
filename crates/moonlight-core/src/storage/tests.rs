use super::*;
use crate::{
    Adapter, BodyCapture, Classification, ComparisonRun, ComparisonSummary, RunInput,
    TargetObservation,
};
use chrono::{TimeZone, Utc};
use std::collections::BTreeMap;
use tempfile::tempdir;

fn body() -> BodyCapture {
    BodyCapture {
        size_bytes: 0,
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        preview: String::new(),
        truncated: false,
    }
}

fn target(latency_ms: u128) -> TargetObservation {
    TargetObservation {
        status: Some(0),
        headers: BTreeMap::new(),
        body: body(),
        stderr: None,
        latency_ms,
        error: None,
    }
}

fn run(
    path: impl Into<String>,
    timestamp_seconds: i64,
    classification: Classification,
    secondary: bool,
) -> ComparisonRun {
    let path = path.into();
    ComparisonRun {
        id: Uuid::new_v4(),
        timestamp: Utc.timestamp_opt(timestamp_seconds, 0).unwrap(),
        adapter: Adapter::Http,
        input: RunInput::Http {
            method: "GET".to_string(),
            path,
            query: None,
        },
        request_headers: BTreeMap::new(),
        request_body: body(),
        primary: target(10),
        candidate: target(20),
        secondary: secondary.then(|| target(30)),
        comparison: ComparisonSummary {
            classification,
            ..Default::default()
        },
    }
}

fn write_runs(path: &std::path::Path, runs: &[ComparisonRun]) {
    let lines = runs
        .iter()
        .map(|run| serde_json::to_string(run).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, format!("{lines}\n")).unwrap();
}

#[tokio::test]
async fn load_creates_parent_directory() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("http-runs.jsonl");

    let _storage = Storage::load(path.clone()).await.unwrap();

    assert!(path.parent().unwrap().exists());
}

#[tokio::test]
async fn run_writer_creates_parent_directory() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("cli-runs.jsonl");

    let writer = RunWriter::open(path.clone()).await.unwrap();
    writer
        .append(&run("writer", 1, Classification::Match, false))
        .await
        .unwrap();
    writer.flush().await.unwrap();

    assert!(path.exists());
}

#[tokio::test]
async fn run_writer_appends_without_loading_existing_files() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cli-runs.jsonl");
    std::fs::write(dir.path().join("corrupt.jsonl"), "not-json\n").unwrap();

    let writer = RunWriter::open(path.clone()).await.unwrap();
    writer
        .append(&run("writer", 1, Classification::Match, false))
        .await
        .unwrap();
    writer.flush().await.unwrap();

    let lines = std::fs::read_to_string(path).unwrap();
    assert_eq!(lines.lines().count(), 1);
}

#[tokio::test]
async fn load_skips_empty_and_corrupt_jsonl_lines() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("http-runs.jsonl");
    let valid = serde_json::to_string(&run("valid", 1, Classification::Match, false)).unwrap();
    std::fs::write(&path, format!("\n{valid}\nnot-json\n\n")).unwrap();

    let storage = Storage::load(path).await.unwrap();
    let runs = storage.list().await;

    assert_eq!(runs.len(), 1);
    assert!(matches!(
        runs[0].input,
        RunInput::Http { ref path, .. } if path == "valid"
    ));
}

#[tokio::test]
async fn list_returns_newest_first() {
    let dir = tempdir().unwrap();
    let storage = Storage::load(dir.path().join("http-runs.jsonl"))
        .await
        .unwrap();
    let first = run("first", 1, Classification::Match, false);
    let second = run("second", 2, Classification::SuspiciousDifference, false);
    storage.insert(first).await.unwrap();
    storage.insert(second).await.unwrap();

    let runs = storage.list().await;

    assert!(matches!(
        runs[0].input,
        RunInput::Http { ref path, .. } if path == "second"
    ));
    assert!(matches!(
        runs[1].input,
        RunInput::Http { ref path, .. } if path == "first"
    ));
}

#[tokio::test]
async fn list_page_returns_newest_first_window() {
    let dir = tempdir().unwrap();
    let storage = Storage::load(dir.path().join("http-runs.jsonl"))
        .await
        .unwrap();
    for index in 0..5 {
        storage
            .insert(run(
                format!("run-{index}"),
                index,
                Classification::Match,
                false,
            ))
            .await
            .unwrap();
    }

    let runs = storage.list_page(2, 1).await;

    assert_eq!(runs.len(), 2);
    assert!(matches!(
        runs[0].input,
        RunInput::Http { ref path, .. } if path == "run-3"
    ));
    assert!(matches!(
        runs[1].input,
        RunInput::Http { ref path, .. } if path == "run-2"
    ));
}

#[tokio::test]
async fn retention_by_max_runs_keeps_newest_active_runs() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("http-runs.jsonl");
    let storage = Storage::load_with_options(
        path.clone(),
        StorageOptions {
            retention_max_runs: Some(2),
            retention_max_bytes: None,
        },
    )
    .await
    .unwrap();
    for index in 0..4 {
        storage
            .insert(run(
                format!("run-{index}"),
                index,
                Classification::Match,
                false,
            ))
            .await
            .unwrap();
    }

    let lines = std::fs::read_to_string(path).unwrap();
    let stored = storage.list().await;

    assert_eq!(lines.lines().count(), 2);
    assert_eq!(stored.len(), 2);
    assert!(matches!(
        stored[0].input,
        RunInput::Http { ref path, .. } if path == "run-3"
    ));
    assert!(matches!(
        stored[1].input,
        RunInput::Http { ref path, .. } if path == "run-2"
    ));
}

#[tokio::test]
async fn concurrent_inserts_are_serialized() {
    let dir = tempdir().unwrap();
    let storage = Storage::load(dir.path().join("http-runs.jsonl"))
        .await
        .unwrap();
    let first = storage.clone();
    let second = storage.clone();

    let (first_result, second_result) = tokio::join!(
        async move {
            first
                .insert(run("first", 1, Classification::Match, false))
                .await
        },
        async move {
            second
                .insert(run("second", 2, Classification::Match, false))
                .await
        }
    );

    first_result.unwrap();
    second_result.unwrap();
    assert_eq!(storage.list().await.len(), 2);
}

#[tokio::test]
async fn load_merges_jsonl_files_in_same_directory() {
    let dir = tempdir().unwrap();
    let http_path = dir.path().join("http-runs.jsonl");
    let cli_path = dir.path().join("cli-runs.jsonl");
    std::fs::write(
        &http_path,
        format!(
            "{}\n",
            serde_json::to_string(&run("http", 1, Classification::Match, false)).unwrap()
        ),
    )
    .unwrap();
    std::fs::write(
        &cli_path,
        format!(
            "{}\n",
            serde_json::to_string(&run("cli", 2, Classification::ReferenceNoise, true)).unwrap()
        ),
    )
    .unwrap();

    let storage = Storage::load(http_path).await.unwrap();
    let stats = storage.stats().await;

    assert_eq!(stats.total_runs, 2);
    assert_eq!(stats.matches, 1);
    assert_eq!(stats.reference_noise, 1);
}

#[tokio::test]
async fn jsonl_reader_reads_only_requested_file() {
    let dir = tempdir().unwrap();
    let requested_path = dir.path().join("cli-runs.jsonl");
    let sibling_path = dir.path().join("http-runs.jsonl");
    write_runs(
        &requested_path,
        &[
            run("cli-1", 1, Classification::Match, false),
            run("cli-2", 2, Classification::SuspiciousDifference, false),
        ],
    );
    write_runs(
        &sibling_path,
        &[run("http", 3, Classification::ReferenceNoise, true)],
    );

    let reader = JsonlStorageReader::new(requested_path);
    let stats = reader.stats().await.unwrap();
    let runs = reader.list_page(None, 0).await.unwrap();

    assert_eq!(stats.total_runs, 2);
    assert_eq!(stats.matches, 1);
    assert_eq!(stats.suspicious_differences, 1);
    assert_eq!(stats.reference_noise, 0);
    assert_eq!(runs.len(), 2);
    assert!(matches!(
        runs[0].input,
        RunInput::Http { ref path, .. } if path == "cli-2"
    ));
}

#[tokio::test]
async fn jsonl_reader_missing_file_is_empty() {
    let dir = tempdir().unwrap();
    let reader = JsonlStorageReader::new(dir.path().join("missing.jsonl"));

    let stats = reader.stats().await.unwrap();
    let list = reader.list_page(Some(10), 0).await.unwrap();
    let found = reader.get(Uuid::new_v4()).await.unwrap();

    assert_eq!(stats.total_runs, 0);
    assert!(stats.latest_runs.is_empty());
    assert!(list.is_empty());
    assert!(found.is_none());
}

#[tokio::test]
async fn jsonl_reader_skips_corrupt_lines() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cli-runs.jsonl");
    let valid = run("valid", 1, Classification::Match, false);
    std::fs::write(
        &path,
        format!("not-json\n{}\n", serde_json::to_string(&valid).unwrap()),
    )
    .unwrap();

    let reader = JsonlStorageReader::new(path);
    let stats = reader.stats().await.unwrap();

    assert_eq!(stats.total_runs, 1);
    assert_eq!(stats.matches, 1);
}

#[tokio::test]
async fn jsonl_reader_pages_newest_first() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cli-runs.jsonl");
    let runs = (0..5)
        .map(|index| run(format!("run-{index}"), index, Classification::Match, false))
        .collect::<Vec<_>>();
    write_runs(&path, &runs);

    let reader = JsonlStorageReader::new(path);
    let page = reader.list_page(Some(2), 1).await.unwrap();

    assert_eq!(page.len(), 2);
    assert!(matches!(
        page[0].input,
        RunInput::Http { ref path, .. } if path == "run-3"
    ));
    assert!(matches!(
        page[1].input,
        RunInput::Http { ref path, .. } if path == "run-2"
    ));
}

#[tokio::test]
async fn jsonl_reader_get_returns_matching_run() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cli-runs.jsonl");
    let first = run("first", 1, Classification::Match, false);
    let second = run("second", 2, Classification::SuspiciousDifference, false);
    let second_id = second.id;
    write_runs(&path, &[first, second]);

    let reader = JsonlStorageReader::new(path);
    let found = reader.get(second_id).await.unwrap().unwrap();

    assert_eq!(found.id, second_id);
    assert!(matches!(
        found.input,
        RunInput::Http { ref path, .. } if path == "second"
    ));
}

#[tokio::test]
async fn jsonl_reader_filters_and_summarizes_runs_from_public_reader() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cli-runs.jsonl");
    let first = run("first", 1, Classification::Match, false);
    let noisy = run("noisy", 2, Classification::ReferenceNoise, true);
    let diff = run("diff", 3, Classification::SuspiciousDifference, false);
    write_runs(&path, &[first, noisy, diff]);

    let reader = JsonlStorageReader::new(path);
    let page = reader
        .filtered_page(
            &RunFilter {
                classification: Some(Classification::SuspiciousDifference),
                adapter: Some(Adapter::Http),
                query: Some("diff".to_string()),
                status: Some(0),
                has_noise: Some(false),
                has_diff: Some(false),
            },
            10,
            0,
        )
        .await
        .unwrap();
    let stats = reader.stats().await.unwrap();

    assert_eq!(page.total, 1);
    assert_eq!(page.items.len(), 1);
    assert!(matches!(
        page.items[0].input,
        RunInput::Http { ref path, .. } if path == "diff"
    ));
    assert_eq!(stats.total_runs, 3);
    assert_eq!(stats.matches, 1);
    assert_eq!(stats.reference_noise, 1);
    assert_eq!(stats.suspicious_differences, 1);
    assert_eq!(stats.latest_runs.len(), 3);
    assert!(matches!(
        stats.latest_runs[0].input,
        RunInput::Http { ref path, .. } if path == "diff"
    ));
}

#[tokio::test]
async fn storage_load_still_scans_directory_for_admin_views() {
    let dir = tempdir().unwrap();
    let http_path = dir.path().join("http-runs.jsonl");
    let cli_path = dir.path().join("cli-runs.jsonl");
    std::fs::write(
        &http_path,
        format!(
            "{}\n",
            serde_json::to_string(&run("http", 1, Classification::Match, false)).unwrap()
        ),
    )
    .unwrap();
    std::fs::write(
        &cli_path,
        format!(
            "{}\n",
            serde_json::to_string(&run("cli", 2, Classification::SuspiciousDifference, false))
                .unwrap()
        ),
    )
    .unwrap();

    let storage = Storage::load(http_path).await.unwrap();
    let stats = storage.stats().await;

    assert_eq!(stats.total_runs, 2);
    assert_eq!(stats.matches, 1);
    assert_eq!(stats.suspicious_differences, 1);
}

#[tokio::test]
async fn refresh_skips_reload_when_jsonl_files_are_unchanged() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("http-runs.jsonl");
    write_runs(&path, &[run("initial", 1, Classification::Match, false)]);
    let storage = Storage::load(path).await.unwrap();

    let refreshed = storage.refresh().await.unwrap();

    assert!(!refreshed);
    assert_eq!(storage.list().await.len(), 1);
}

#[tokio::test]
async fn refresh_loads_new_runs_when_write_file_changes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("http-runs.jsonl");
    write_runs(&path, &[run("initial", 1, Classification::Match, false)]);
    let storage = Storage::load(path.clone()).await.unwrap();
    write_runs(
        &path,
        &[
            run("initial", 1, Classification::Match, false),
            run("changed", 2, Classification::SuspiciousDifference, false),
        ],
    );

    let refreshed = storage.refresh().await.unwrap();
    let runs = storage.list().await;

    assert!(refreshed);
    assert_eq!(runs.len(), 2);
    assert!(matches!(
        runs[0].input,
        RunInput::Http { ref path, .. } if path == "changed"
    ));
}

#[tokio::test]
async fn refresh_loads_new_runs_when_sibling_jsonl_file_changes() {
    let dir = tempdir().unwrap();
    let http_path = dir.path().join("http-runs.jsonl");
    let cli_path = dir.path().join("cli-runs.jsonl");
    write_runs(&http_path, &[run("http", 1, Classification::Match, false)]);
    let storage = Storage::load(http_path).await.unwrap();
    write_runs(
        &cli_path,
        &[run("cli", 2, Classification::ReferenceNoise, true)],
    );

    let refreshed = storage.refresh().await.unwrap();
    let stats = storage.stats().await;

    assert!(refreshed);
    assert_eq!(stats.total_runs, 2);
    assert_eq!(stats.reference_noise, 1);
}

#[tokio::test]
async fn stats_limits_latest_runs_to_20() {
    let dir = tempdir().unwrap();
    let storage = Storage::load(dir.path().join("http-runs.jsonl"))
        .await
        .unwrap();
    for index in 0..25 {
        storage
            .insert(run(
                format!("run-{index}"),
                index,
                Classification::Match,
                false,
            ))
            .await
            .unwrap();
    }

    let stats = storage.stats().await;

    assert_eq!(stats.total_runs, 25);
    assert_eq!(stats.latest_runs.len(), 20);
    assert!(matches!(
        stats.latest_runs[0].input,
        RunInput::Http { ref path, .. } if path == "run-24"
    ));
    assert!(matches!(
        stats.latest_runs[19].input,
        RunInput::Http { ref path, .. } if path == "run-5"
    ));
}

#[tokio::test]
async fn stats_handles_missing_secondary_latencies() {
    let dir = tempdir().unwrap();
    let storage = Storage::load(dir.path().join("http-runs.jsonl"))
        .await
        .unwrap();
    storage
        .insert(run("primary-candidate", 1, Classification::Match, false))
        .await
        .unwrap();

    let stats = storage.stats().await;

    assert_eq!(stats.total_runs, 1);
    assert_eq!(stats.latency.primary_avg_ms, 10.0);
    assert_eq!(stats.latency.candidate_avg_ms, 20.0);
    assert_eq!(stats.latency.secondary_avg_ms, None);
}

#[tokio::test]
async fn retention_rewrite_replaces_file_atomically() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("http-runs.jsonl");
    let storage = Storage::load_with_options(
        path.clone(),
        StorageOptions {
            retention_max_runs: Some(2),
            retention_max_bytes: None,
        },
    )
    .await
    .unwrap();
    for index in 0..5 {
        storage
            .insert(run(
                format!("run-{index}"),
                index,
                Classification::Match,
                false,
            ))
            .await
            .unwrap();
    }

    let lines = std::fs::read_to_string(&path).unwrap();
    let temp_files = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("tmp"))
        .count();

    assert_eq!(lines.lines().count(), 2);
    assert_eq!(temp_files, 0);
    assert!(lines.contains("run-4"));
    assert!(lines.contains("run-3"));
    assert!(!lines.contains("run-2"));
}

#[tokio::test]
async fn retention_skip_rewrite_when_active_runs_are_already_within_limits() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("http-runs.jsonl");
    write_runs(
        &path,
        &[
            run("first", 1, Classification::Match, false),
            run("second", 2, Classification::Match, false),
        ],
    );
    let before = std::fs::read_to_string(&path).unwrap();
    let storage = Storage::load_with_options(
        path.clone(),
        StorageOptions {
            retention_max_runs: Some(10),
            retention_max_bytes: None,
        },
    )
    .await
    .unwrap();

    storage.apply_retention().await.unwrap();

    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(after, before);
}
