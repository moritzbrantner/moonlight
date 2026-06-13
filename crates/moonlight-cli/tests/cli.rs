use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::Instant,
};
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

fn read_jsonl(path: &str) -> Vec<Value> {
    let content = fs::read_to_string(path).expect("storage should be readable");
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("storage line should be JSON"))
        .collect()
}

fn write_batch_cases(path: &Path, cases: &[Value]) {
    let lines = cases
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{lines}\n")).unwrap();
}

#[test]
fn help_prints_command_surface() {
    cli()
        .arg("--help")
        .assert()
        .success()
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

#[test]
fn batch_records_multiple_cases() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    fs::write(
        &input_path,
        [
            r#"{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":42}'"}"#,
            r#"{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":43}'"}"#,
            r#"{"primary":"printf '%s\n' '{\"region\":\"a\",\"value\":1}'","candidate":"printf '%s\n' '{\"region\":\"a\",\"value\":1}'","secondary":"printf '%s\n' '{\"region\":\"b\",\"value\":1}'"}"#,
        ]
        .join("\n"),
    )
    .unwrap();

    let summary = read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);

    assert_eq!(summary["total_runs"], 3);
    assert_eq!(summary["matches"], 1);
    assert_eq!(summary["suspicious_differences"], 1);
    assert_eq!(summary["reference_noise"], 1);
    assert_eq!(read_jsonl(&storage).len(), 3);
    dir.close().unwrap();
}

#[test]
fn batch_reads_stdin() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let mut child = cli()
        .args(["batch", "--storage-path", &storage, "--jobs", "1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn moonlight-cli");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(
            br#"{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":42}'"}"#,
        )
        .unwrap();
    let output = child.wait_with_output().expect("wait for moonlight-cli");
    assert!(
        output.status.success(),
        "moonlight-cli failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(&output.stdout).expect("summary should be JSON");

    assert_eq!(summary["total_runs"], 1);
    assert_eq!(summary["matches"], 1);
    assert_eq!(read_jsonl(&storage).len(), 1);
    dir.close().unwrap();
}

#[test]
fn batch_quiet_suppresses_stdout() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    fs::write(
        &input_path,
        r#"{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":42}'"}"#,
    )
    .unwrap();

    cli()
        .args([
            "batch",
            "--input",
            input_path.to_str().unwrap(),
            "--storage-path",
            &storage,
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert_eq!(read_jsonl(&storage).len(), 1);
    dir.close().unwrap();
}

#[test]
fn batch_emit_runs_outputs_jsonl_records() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    fs::write(
        &input_path,
        [
            r#"{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":42}'"}"#,
            r#"{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":43}'"}"#,
        ]
        .join("\n"),
    )
    .unwrap();

    let output = cli()
        .args([
            "batch",
            "--input",
            input_path.to_str().unwrap(),
            "--storage-path",
            &storage,
            "--emit-runs",
            "--jobs",
            "1",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let lines = String::from_utf8(output).unwrap();
    let records: Vec<Value> = lines
        .lines()
        .map(|line| serde_json::from_str(line).expect("emitted line should be JSON"))
        .collect();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["comparison"]["classification"], "match");
    assert_eq!(
        records[1]["comparison"]["classification"],
        "suspicious_difference"
    );
    dir.close().unwrap();
}

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

#[test]
fn batch_argv_records_match() {
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

    read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);
    let records = read_jsonl(&storage);

    assert_eq!(records[0]["comparison"]["classification"], "match");
    assert!(records[0]["input"]["primary_command"]
        .as_str()
        .unwrap()
        .starts_with("printf "));
    dir.close().unwrap();
}

#[test]
fn batch_argv_records_shell_escaped_display_strings() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[serde_json::json!({
            "primary_argv": ["printf", "%s", "a'b"],
            "candidate_argv": ["printf", "%s", "a'b"]
        })],
    );

    read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);
    let records = read_jsonl(&storage);

    assert_eq!(records[0]["comparison"]["classification"], "match");
    assert_eq!(
        records[0]["input"]["primary_command"],
        "printf '%s' 'a'\\''b'"
    );
    dir.close().unwrap();
}

#[test]
fn batch_argv_candidate_diff_records_suspicious_difference() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[serde_json::json!({
            "primary_argv": ["printf", "%s", "{\"value\":42}"],
            "candidate_argv": ["printf", "%s", "{\"value\":43}"]
        })],
    );

    read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);
    let records = read_jsonl(&storage);

    assert_eq!(
        records[0]["comparison"]["classification"],
        "suspicious_difference"
    );
    dir.close().unwrap();
}

#[test]
fn batch_default_and_custom_compare_config_classify_expected() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    write_batch_cases(
        &input_path,
        &[
            serde_json::json!({
                "primary": "printf '%s' '{\"value\":42}'",
                "candidate": "printf '%s' '{\"value\":42}'"
            }),
            serde_json::json!({
                "primary": "printf '%s' '{\"dynamic\":\"one\",\"stable\":true}'",
                "candidate": "printf '%s' '{\"dynamic\":\"two\",\"stable\":true}'",
                "ignored_json_paths": ["$.dynamic"]
            }),
        ],
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

    assert_eq!(summary["total_runs"], 2);
    assert_eq!(summary["matches"], 2);
    dir.close().unwrap();
}

#[test]
fn batch_invalid_json_exits_nonzero_and_writes_no_records() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    fs::write(&input_path, "not-json\n").unwrap();

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
        .stderr(predicate::str::contains("line 1"));

    assert!(!Path::new(&storage).exists());
    dir.close().unwrap();
}

#[test]
fn batch_jobs_one_is_valid() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    fs::write(
        &input_path,
        r#"{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":42}'"}"#,
    )
    .unwrap();

    let summary = read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);

    assert_eq!(summary["jobs"], 1);
    assert_eq!(summary["total_runs"], 1);
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
fn run_captures_large_stdout_and_stderr_without_deadlock() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let command = "python3 -c 'import sys; sys.stdout.write(\"a\" * 131072); sys.stderr.write(\"e\" * 131072)'";

    let record = run_record(&storage, &["--primary", command, "--candidate", command]);

    assert_eq!(record["comparison"]["classification"], "match");
    assert_eq!(record["primary"]["body"]["size_bytes"], 131072);
    assert_eq!(record["primary"]["stderr"]["size_bytes"], 131072);
    dir.close().unwrap();
}

#[test]
fn run_streamed_candidate_body_diff_still_records_diff() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary = "python3 -c 'print(\"a\" * 32768, end=\"\")'";
    let candidate = "python3 -c 'print(\"b\" * 32768, end=\"\")'";

    let record = run_record(&storage, &["--primary", primary, "--candidate", candidate]);

    assert_eq!(
        record["comparison"]["classification"],
        "suspicious_difference"
    );
    assert_eq!(
        record["comparison"]["noise_filtered_diffs"][0]["kind"],
        "body"
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
