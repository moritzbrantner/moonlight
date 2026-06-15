use crate::cli_support::{cli, read_jsonl, run_record, storage_path};
use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

fn argv(args: &[&str]) -> String {
    serde_json::to_string(args).unwrap()
}

#[test]
fn run_accepts_primary_and_candidate_argv_json() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = argv(&["printf", "%s\n", r#"{"value":42}"#]);
    let candidate = argv(&["printf", "%s\n", r#"{"value":42}"#]);

    let record = run_record(
        &storage,
        &["--primary-argv", &primary, "--candidate-argv", &candidate],
    );

    assert_eq!(record["comparison"]["classification"], "match");
    assert_eq!(record["primary"]["status"], 0);
    assert_eq!(record["candidate"]["status"], 0);
    dir.close().unwrap();
}

#[test]
fn run_accepts_secondary_argv_json() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = argv(&["printf", "%s", r#"{"region":"a","value":1}"#]);
    let candidate = argv(&["printf", "%s", r#"{"region":"a","value":1}"#]);
    let secondary = argv(&["printf", "%s", r#"{"region":"b","value":1}"#]);

    let record = run_record(
        &storage,
        &[
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--secondary-argv",
            &secondary,
        ],
    );

    assert_eq!(record["comparison"]["classification"], "reference_noise");
    dir.close().unwrap();
}

#[test]
fn run_rejects_invalid_argv_json() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let candidate = argv(&["printf", "%s", "ok"]);

    cli()
        .args([
            "run",
            "--storage-path",
            &storage,
            "--primary-argv",
            "not-json",
            "--candidate-argv",
            &candidate,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--primary-argv"))
        .stderr(predicate::str::contains("JSON string array"));

    dir.close().unwrap();
}

#[test]
fn run_rejects_empty_argv_array() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let candidate = argv(&["printf", "%s", "ok"]);

    cli()
        .args([
            "run",
            "--storage-path",
            &storage,
            "--primary-argv",
            "[]",
            "--candidate-argv",
            &candidate,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--primary-argv must not be empty"));

    dir.close().unwrap();
}

#[test]
fn run_rejects_blank_argv_executable() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = argv(&[" ", "ok"]);
    let candidate = argv(&["printf", "%s", "ok"]);

    cli()
        .args([
            "run",
            "--storage-path",
            &storage,
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--primary-argv command must not be empty",
        ));

    dir.close().unwrap();
}

#[test]
fn run_rejects_primary_shell_and_argv_together() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = argv(&["printf", "%s", "ok"]);
    let candidate = argv(&["printf", "%s", "ok"]);

    cli()
        .args([
            "run",
            "--storage-path",
            &storage,
            "--primary",
            "printf ok",
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "exactly one of --primary or --primary-argv",
        ));

    dir.close().unwrap();
}

#[test]
fn run_rejects_missing_primary_form() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let candidate = argv(&["printf", "%s", "ok"]);

    cli()
        .args([
            "run",
            "--storage-path",
            &storage,
            "--candidate-argv",
            &candidate,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("primary command form is required"))
        .stderr(predicate::str::contains("--primary"));

    dir.close().unwrap();
}

#[test]
fn run_argv_records_shell_escaped_display_strings() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = argv(&["printf", "%s", "a'b"]);
    let candidate = argv(&["printf", "%s", "a'b"]);

    let record = run_record(
        &storage,
        &["--primary-argv", &primary, "--candidate-argv", &candidate],
    );

    assert_eq!(record["comparison"]["classification"], "match");
    assert_eq!(record["input"]["primary_command"], "printf '%s' 'a'\\''b'");
    assert_eq!(
        read_jsonl(&storage)[0]["input"]["candidate_command"],
        "printf '%s' 'a'\\''b'"
    );
    dir.close().unwrap();
}

#[test]
fn run_argv_respects_serial_targets() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let order_path = dir.path().join("order.txt");
    let path_literal = serde_json::to_string(order_path.to_str().unwrap()).unwrap();
    let primary_code =
        format!("from pathlib import Path; Path({path_literal}).open('a').write('primary\\n')");
    let candidate_code =
        format!("from pathlib import Path; Path({path_literal}).open('a').write('candidate\\n')");
    let primary = argv(&["python3", "-c", &primary_code]);
    let candidate = argv(&["python3", "-c", &candidate_code]);

    run_record(
        &storage,
        &[
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--serial-targets",
        ],
    );

    assert_eq!(
        fs::read_to_string(order_path).unwrap(),
        "primary\ncandidate\n"
    );
    dir.close().unwrap();
}

#[test]
fn run_argv_compact_and_quiet_keep_existing_output_semantics() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = argv(&["printf", "%s", r#"{"value":42}"#]);
    let candidate = argv(&["printf", "%s", r#"{"value":42}"#]);

    let output = cli()
        .arg("run")
        .arg("--storage-path")
        .arg(&storage)
        .args([
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--compact",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let record: Value = serde_json::from_str(stdout.trim_end()).expect("compact JSON");
    assert_eq!(stdout.lines().count(), 1);
    assert_eq!(record["comparison"]["classification"], "match");

    cli()
        .arg("run")
        .arg("--storage-path")
        .arg(&storage)
        .args([
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--compact",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert_eq!(read_jsonl(&storage).len(), 2);
    dir.close().unwrap();
}
