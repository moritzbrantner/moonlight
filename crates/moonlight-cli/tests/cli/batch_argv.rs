use crate::cli_support::{cli, read_json, storage_path, write_batch_cases};
use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use std::path::Path;

#[test]
fn batch_accepts_argv_cases() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[serde_json::json!({
            "primary_argv": ["printf", "%s", "{\"value\":42}"],
            "candidate_argv": ["printf", "%s", "{\"value\":42}"]
        })],
    );

    let summary = read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);

    assert_eq!(summary["total_runs"], 1);
    assert_eq!(summary["matches"], 1);
    dir.close().unwrap();
}

#[test]
fn batch_accepts_secondary_argv() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[serde_json::json!({
            "primary_argv": ["printf", "%s", "{\"region\":\"a\",\"value\":1}"],
            "candidate_argv": ["printf", "%s", "{\"region\":\"a\",\"value\":1}"],
            "secondary_argv": ["printf", "%s", "{\"region\":\"b\",\"value\":1}"]
        })],
    );

    let summary = read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);

    assert_eq!(summary["reference_noise"], 1);
    dir.close().unwrap();
}

#[test]
fn batch_rejects_empty_argv() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[serde_json::json!({
            "primary_argv": [],
            "candidate_argv": ["printf", "%s", "{\"value\":42}"]
        })],
    );

    cli()
        .args([
            "batch",
            "--input",
            input_path.to_str().unwrap(),
            "--storage-path",
            &storage,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"))
        .stderr(predicate::str::contains("primary_argv must not be empty"));

    assert!(!Path::new(&storage).exists());
    dir.close().unwrap();
}

#[test]
fn batch_rejects_blank_argv_executable() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[serde_json::json!({
            "primary_argv": [" ", "ok"],
            "candidate_argv": ["printf", "%s", "ok"]
        })],
    );

    cli()
        .args([
            "batch",
            "--input",
            input_path.to_str().unwrap(),
            "--storage-path",
            &storage,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"))
        .stderr(predicate::str::contains(
            "primary_argv command must not be empty",
        ));

    assert!(!Path::new(&storage).exists());
    dir.close().unwrap();
}

#[test]
fn batch_rejects_primary_string_and_primary_argv_together() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[serde_json::json!({
            "primary": "printf ok",
            "primary_argv": ["printf", "ok"],
            "candidate_argv": ["printf", "ok"]
        })],
    );

    cli()
        .args([
            "batch",
            "--input",
            input_path.to_str().unwrap(),
            "--storage-path",
            &storage,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"))
        .stderr(predicate::str::contains(
            "exactly one of primary or primary_argv",
        ));

    dir.close().unwrap();
}

#[test]
fn batch_rejects_missing_primary_command_form() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[serde_json::json!({
            "candidate_argv": ["printf", "%s", "{\"value\":42}"]
        })],
    );

    cli()
        .args([
            "batch",
            "--input",
            input_path.to_str().unwrap(),
            "--storage-path",
            &storage,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"))
        .stderr(predicate::str::contains("primary command form is required"));

    dir.close().unwrap();
}
