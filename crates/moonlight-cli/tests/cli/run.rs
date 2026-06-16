use crate::cli_support::{cli, json_command, read_jsonl, run_record, storage_path};
use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use serde_json::Value;
use std::{fs, time::Instant};

#[test]
fn help_prints_command_surface() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: moonlight"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("batch"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn run_requires_primary() {
    cli()
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--primary"));
}

#[test]
fn run_requires_candidate() {
    cli()
        .args(["run", "--primary", "printf ok"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--candidate"));
}

#[test]
fn run_records_match() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = json_command(r#"{"value":42}"#);
    let candidate = json_command(r#"{"value":42}"#);

    let record = run_record(
        &storage,
        &["--primary", &primary, "--candidate", &candidate],
    );

    assert_eq!(record["adapter"], "cli");
    assert_eq!(record["input"]["primary_command"], primary);
    assert_eq!(record["input"]["candidate_command"], candidate);
    assert_eq!(record["primary"]["status"], 0);
    assert_eq!(record["candidate"]["status"], 0);
    assert_eq!(record["comparison"]["classification"], "match");
    dir.close().unwrap();
}

#[test]
fn run_uses_configured_targets_and_storage() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let config = dir.path().join("moonlight.conf");
    fs::write(
        &config,
        format!(
            "[storage]\npath = \"{}\"\n\n[cli.run]\nprimary = \"printf ok\"\ncandidate = \"printf ok\"\nquiet = true\n",
            storage.replace('\\', "\\\\")
        ),
    )
    .unwrap();

    cli()
        .arg("--config")
        .arg(&config)
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let records = read_jsonl(&storage);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["comparison"]["classification"], "match");
    dir.close().unwrap();
}

#[test]
fn run_target_flags_override_configured_targets_by_role() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let config = dir.path().join("moonlight.conf");
    fs::write(
        &config,
        format!(
            "[storage]\npath = \"{}\"\n\n[cli.run]\nprimary = \"printf config\"\ncandidate = \"printf config\"\n",
            storage.replace('\\', "\\\\")
        ),
    )
    .unwrap();

    let output = cli()
        .arg("--config")
        .arg(&config)
        .arg("run")
        .args(["--candidate", "printf flag"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let record: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(record["input"]["primary_command"], "printf config");
    assert_eq!(record["input"]["candidate_command"], "printf flag");
    assert_eq!(
        record["comparison"]["classification"],
        "suspicious_difference"
    );
    dir.close().unwrap();
}

#[test]
fn run_quiet_writes_storage_without_stdout() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = json_command(r#"{"value":42}"#);
    let candidate = json_command(r#"{"value":42}"#);

    cli()
        .arg("run")
        .arg("--storage-path")
        .arg(&storage)
        .args(["--primary", &primary, "--candidate", &candidate, "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let records = read_jsonl(&storage);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["comparison"]["classification"], "match");
    dir.close().unwrap();
}

#[test]
fn run_compact_outputs_single_line_json() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = json_command(r#"{"value":42}"#);
    let candidate = json_command(r#"{"value":42}"#);

    let output = cli()
        .arg("run")
        .arg("--storage-path")
        .arg(&storage)
        .args([
            "--primary",
            &primary,
            "--candidate",
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
    assert_eq!(read_jsonl(&storage).len(), 1);
    dir.close().unwrap();
}

#[test]
fn run_quiet_with_compact_writes_storage_without_stdout() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = json_command(r#"{"value":42}"#);
    let candidate = json_command(r#"{"value":42}"#);

    cli()
        .arg("run")
        .arg("--storage-path")
        .arg(&storage)
        .args([
            "--primary",
            &primary,
            "--candidate",
            &candidate,
            "--compact",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert_eq!(read_jsonl(&storage).len(), 1);
    dir.close().unwrap();
}

#[test]
fn run_serial_targets_preserves_order() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let order_path = dir.path().join("order.txt");
    let order = order_path.to_string_lossy();
    let primary = format!("printf '%s\\n' primary >> '{order}'");
    let candidate = format!("printf '%s\\n' candidate >> '{order}'");

    run_record(
        &storage,
        &[
            "--primary",
            &primary,
            "--candidate",
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
fn run_parallel_targets_is_default() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = "sleep 0.5; printf '%s\\n' ok";
    let candidate = "sleep 0.5; printf '%s\\n' ok";

    let started = Instant::now();
    let record = run_record(&storage, &["--primary", primary, "--candidate", candidate]);
    let elapsed = started.elapsed();

    assert_eq!(record["comparison"]["classification"], "match");
    assert!(
        elapsed.as_millis() < 900,
        "expected parallel target execution, elapsed {elapsed:?}"
    );
    dir.close().unwrap();
}

#[test]
fn run_records_suspicious_difference() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = json_command(r#"{"value":42}"#);
    let candidate = json_command(r#"{"value":43}"#);

    let record = run_record(
        &storage,
        &["--primary", &primary, "--candidate", &candidate],
    );

    assert_eq!(
        record["comparison"]["classification"],
        "suspicious_difference"
    );
    assert_eq!(
        record["comparison"]["noise_filtered_diffs"][0]["path"],
        "$.value"
    );
    dir.close().unwrap();
}
