use crate::cli_support::{json_command, run_record, storage_path};
use assert_fs::TempDir;

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
fn run_custom_ignore_json_path_extends_defaults() {
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
            "--ignore-json-path",
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
