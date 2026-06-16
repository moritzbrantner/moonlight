use crate::cli_support::{cli, read_json, read_jsonl, storage_path, write_batch_cases};
use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use serde_json::Value;
use std::{collections::BTreeSet, fs, io::Write, path::Path, process::Stdio};

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
fn batch_writer_writes_all_records_exactly_once() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    let cases = (0..16)
        .map(|index| {
            serde_json::json!({
                "primary_argv": ["printf", "%s", format!("case-{index}")],
                "candidate_argv": ["printf", "%s", format!("case-{index}")]
            })
        })
        .collect::<Vec<_>>();
    write_batch_cases(&input_path, &cases);

    let summary = read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "8",
    ]);
    let records = read_jsonl(&storage);
    let unique_commands = records
        .iter()
        .map(|record| {
            record["input"]["primary_command"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(summary["total_runs"], 16);
    assert_eq!(records.len(), 16);
    assert_eq!(unique_commands.len(), 16);
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
fn batch_preserves_completion_order_for_storage_and_emit_runs() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    fs::write(
        &input_path,
        [
            r#"{"primary":"sleep 0.5; printf '%s' slow","candidate":"sleep 0.5; printf '%s' slow"}"#,
            r#"{"primary":"printf '%s' fast","candidate":"printf '%s' fast"}"#,
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
            "2",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let emitted = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("valid JSONL"))
        .collect::<Vec<_>>();
    let stored = read_jsonl(&storage);

    assert_eq!(emitted.len(), 2);
    assert_eq!(stored.len(), 2);
    assert_eq!(emitted[0]["input"]["primary_command"], "printf '%s' fast");
    assert_eq!(stored[0]["input"]["primary_command"], "printf '%s' fast");
    assert_eq!(
        emitted[0]["id"], stored[0]["id"],
        "stdout and storage should use the same completion order"
    );
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

#[cfg(unix)]
#[test]
fn batch_writer_errors_propagate_as_command_failure() {
    let dir = TempDir::new().unwrap();
    let input_path = dir.path().join("cases.jsonl");
    fs::write(
        &input_path,
        r#"{"primary_argv":["printf","%s","ok"],"candidate_argv":["printf","%s","ok"]}"#,
    )
    .unwrap();

    cli()
        .args([
            "batch",
            "--input",
            input_path.to_str().unwrap(),
            "--storage-path",
            "/dev/full",
            "--quiet",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("batch writer failed"));

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
                "ignore_json_paths": ["$.dynamic"]
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
