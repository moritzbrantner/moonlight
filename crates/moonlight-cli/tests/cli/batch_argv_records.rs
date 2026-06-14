use crate::cli_support::{read_json, read_jsonl, storage_path, write_batch_cases};
use assert_fs::TempDir;

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
