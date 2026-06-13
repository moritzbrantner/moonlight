use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use serde_json::Value;
use std::{path::Path, process::Command};
use uuid::Uuid;

fn cli() -> Command {
    Command::cargo_bin("moonlight-cli").expect("moonlight-cli binary")
}

fn storage_path(dir: &TempDir) -> String {
    dir.path().join("runs.jsonl").to_string_lossy().into_owned()
}

fn json_command(value: &str) -> String {
    format!("printf '%s\\n' '{}'", value)
}

fn run_record(storage: &str, args: &[&str]) -> Value {
    let output = cli()
        .arg("run")
        .arg("--storage-path")
        .arg(storage)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("run stdout should be JSON")
}

fn read_json(args: &[&str]) -> Value {
    let output = cli()
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("stdout should be JSON")
}

#[test]
fn help_prints_command_surface() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("run"))
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

#[test]
fn run_filters_reference_noise() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = json_command(r#"{"region":"a","value":1}"#);
    let candidate = json_command(r#"{"region":"a","value":1}"#);
    let secondary = json_command(r#"{"region":"b","value":1}"#);

    let record = run_record(
        &storage,
        &[
            "--primary",
            &primary,
            "--candidate",
            &candidate,
            "--secondary",
            &secondary,
        ],
    );

    assert_eq!(record["comparison"]["classification"], "reference_noise");
    assert_eq!(
        record["comparison"]["reference_noise"][0]["path"],
        "$.region"
    );
    dir.close().unwrap();
}

#[test]
fn run_records_suspicious_with_noise() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = json_command(r#"{"region":"a","total":42}"#);
    let candidate = json_command(r#"{"region":"a","total":99}"#);
    let secondary = json_command(r#"{"region":"b","total":42}"#);

    let record = run_record(
        &storage,
        &[
            "--primary",
            &primary,
            "--candidate",
            &candidate,
            "--secondary",
            &secondary,
        ],
    );

    assert_eq!(
        record["comparison"]["classification"],
        "suspicious_with_noise"
    );
    assert_eq!(
        record["comparison"]["noise_filtered_diffs"][0]["path"],
        "$.total"
    );
    dir.close().unwrap();
}

#[test]
fn run_ignores_default_json_ids() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary =
        json_command(r#"{"id":"a","requestId":"b","traceId":"c","timestamp":"one","value":42}"#);
    let candidate =
        json_command(r#"{"id":"d","requestId":"e","traceId":"f","timestamp":"two","value":42}"#);

    let record = run_record(
        &storage,
        &["--primary", &primary, "--candidate", &candidate],
    );

    assert_eq!(record["comparison"]["classification"], "match");
    dir.close().unwrap();
}

#[test]
fn run_custom_ignored_json_path_overrides_defaults() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = json_command(r#"{"dynamic":"one","stable":true}"#);
    let candidate = json_command(r#"{"dynamic":"two","stable":true}"#);

    let record = run_record(
        &storage,
        &[
            "--primary",
            &primary,
            "--candidate",
            &candidate,
            "--ignored-json-path",
            "$.dynamic",
        ],
    );

    assert_eq!(record["comparison"]["classification"], "match");
    dir.close().unwrap();
}

#[test]
fn run_captures_stderr_stream() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = "printf '%s\\n' ok; printf '%s' primary-error >&2";
    let candidate = "printf '%s\\n' ok; printf '%s' primary-error >&2";

    let record = run_record(&storage, &["--primary", primary, "--candidate", candidate]);

    assert!(record["primary"]["stderr"]["sha256"].is_string());
    assert_eq!(record["primary"]["stderr"]["preview"], "primary-error");
    dir.close().unwrap();
}

#[test]
fn run_can_ignore_stderr_diffs() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = "printf '%s\\n' ok; printf '%s' primary-error >&2";
    let candidate = "printf '%s\\n' ok; printf '%s' candidate-error >&2";

    let record = run_record(
        &storage,
        &[
            "--primary",
            primary,
            "--candidate",
            candidate,
            "--ignore-stderr",
        ],
    );

    assert_eq!(record["comparison"]["classification"], "match");
    dir.close().unwrap();
}

#[test]
fn run_records_exit_status_diff() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = "printf '%s\\n' ok";
    let candidate = "printf '%s\\n' ok; exit 2";

    let record = run_record(&storage, &["--primary", primary, "--candidate", candidate]);

    assert_eq!(record["candidate"]["status"], 2);
    assert_eq!(
        record["comparison"]["classification"],
        "suspicious_difference"
    );
    assert!(record["candidate"]["error"].is_null());
    dir.close().unwrap();
}

#[cfg(unix)]
#[test]
fn run_records_signal_as_target_error() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = "printf '%s\\n' ok";
    let candidate = "kill -TERM $$";

    let record = run_record(&storage, &["--primary", primary, "--candidate", candidate]);

    assert_eq!(record["comparison"]["classification"], "target_error");
    assert!(record["candidate"]["error"]
        .as_str()
        .unwrap()
        .contains("terminated by signal"));
    dir.close().unwrap();
}

#[test]
fn run_truncates_large_body_preview() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = "printf '%s' abcdefghij";
    let candidate = "printf '%s' abcdefghij";

    let record = run_record(
        &storage,
        &[
            "--primary",
            primary,
            "--candidate",
            candidate,
            "--max-body-capture-bytes",
            "5",
        ],
    );

    assert_eq!(record["primary"]["body"]["truncated"], true);
    assert_eq!(record["primary"]["body"]["size_bytes"], 10);
    assert_eq!(
        record["primary"]["body"]["preview"].as_str().unwrap().len(),
        5
    );
    dir.close().unwrap();
}

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
