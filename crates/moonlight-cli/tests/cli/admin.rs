use crate::cli_support::{cli, json_command, read_json, run_record, storage_path};
use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use std::path::Path;
use uuid::Uuid;

#[test]
fn list_lists_runs_newest_first() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let first = run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":1}"#),
            "--candidate",
            &json_command(r#"{"value":1}"#),
        ],
    );
    let second = run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":2}"#),
            "--candidate",
            &json_command(r#"{"value":2}"#),
        ],
    );

    let runs = read_json(&["list", "--storage-path", &storage]);

    assert_eq!(runs.as_array().unwrap().len(), 2);
    assert_eq!(runs[0]["id"], second["id"]);
    assert_eq!(runs[1]["id"], first["id"]);
    dir.close().unwrap();
}

#[test]
fn stats_summarizes_classifications_and_latencies() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":42}"#),
            "--candidate",
            &json_command(r#"{"value":42}"#),
        ],
    );
    run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":42}"#),
            "--candidate",
            &json_command(r#"{"value":43}"#),
        ],
    );
    run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"region":"a","value":1}"#),
            "--candidate",
            &json_command(r#"{"region":"a","value":1}"#),
            "--secondary",
            &json_command(r#"{"region":"b","value":1}"#),
        ],
    );

    let stats = read_json(&["stats", "--storage-path", &storage]);

    assert_eq!(stats["total_runs"], 3);
    assert_eq!(stats["matches"], 1);
    assert_eq!(stats["suspicious_differences"], 1);
    assert_eq!(stats["reference_noise"], 1);
    assert!(stats["latency"]["primary_avg_ms"].is_number());
    assert!(stats["latency"]["candidate_avg_ms"].is_number());
    assert!(stats["latency"]["secondary_avg_ms"].is_number());
    dir.close().unwrap();
}

#[test]
fn show_returns_full_record() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let record = run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":42}"#),
            "--candidate",
            &json_command(r#"{"value":43}"#),
        ],
    );
    let id = record["id"].as_str().unwrap();

    let shown = read_json(&["show", id, "--storage-path", &storage]);

    assert_eq!(shown["id"], record["id"]);
    assert_eq!(shown["comparison"], record["comparison"]);
    dir.close().unwrap();
}

#[test]
fn show_missing_id_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let missing = Uuid::new_v4().to_string();

    cli()
        .args(["show", &missing, "--storage-path", &storage])
        .assert()
        .failure()
        .stderr(predicate::str::contains("was not found"));
    dir.close().unwrap();
}

#[allow(dead_code)]
fn _assert_path_is_outside_repo(path: &Path) {
    assert!(path.is_absolute());
}
