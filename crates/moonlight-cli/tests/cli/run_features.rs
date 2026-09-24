use crate::cli_support::{json_command, run_record, storage_path};
use assert_fs::TempDir;
use std::fs;

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
fn run_records_timeout_as_target_error() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);

    let record = run_record(
        &storage,
        &[
            "--primary",
            "printf '%s\n' ok",
            "--candidate",
            "sleep 1; printf '%s\n' ok",
            "--target-timeout-ms",
            "25",
        ],
    );

    assert_eq!(record["comparison"]["classification"], "target_error");
    assert!(record["candidate"]["error"]
        .as_str()
        .unwrap()
        .contains("timed out"));
    dir.close().unwrap();
}

#[test]
fn run_redacts_target_previews_and_complete_persisted_record() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let primary_path = dir.path().join("primary.json");
    let candidate_path = dir.path().join("candidate.json");
    fs::write(&primary_path, r#"{"visible":1}"#).unwrap();
    fs::write(
        &candidate_path,
        r#"{"visible":1,"token":"AUDIT_SENTINEL"}"#,
    )
    .unwrap();

    let primary = serde_json::to_string(&[
        "cat",
        primary_path.to_str().unwrap(),
    ])
    .unwrap();
    let candidate = serde_json::to_string(&[
        "cat",
        candidate_path.to_str().unwrap(),
    ])
    .unwrap();

    let record = run_record(
        &storage,
        &[
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--redact-json-path",
            "$.token",
        ],
    );
    let stdout_record = serde_json::to_string(&record).unwrap();
    let persisted = fs::read_to_string(&storage).unwrap();

    assert_eq!(
        record["comparison"]["classification"],
        "suspicious_difference"
    );
    assert!(record["candidate"]["body"]["preview"]
        .as_str()
        .unwrap()
        .contains("[redacted]"));
    assert!(!stdout_record.contains("AUDIT_SENTINEL"));
    assert!(!persisted.contains("AUDIT_SENTINEL"));
    dir.close().unwrap();
}

#[cfg(unix)]
#[test]
fn run_timeout_bounds_descendant_pipe_lifecycle_and_cleans_descendant() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let pid_path = dir.path().join("descendant.pid");
    let path_literal = serde_json::to_string(pid_path.to_str().unwrap()).unwrap();
    let code = format!(
        "from pathlib import Path; import subprocess; p=subprocess.Popen(['sleep','3']); Path({path_literal}).write_text(str(p.pid))"
    );
    let primary = serde_json::to_string(&["printf", "%s", "ok"]).unwrap();
    let candidate = serde_json::to_string(&["python3", "-c", &code]).unwrap();

    let started = std::time::Instant::now();
    let record = run_record(
        &storage,
        &[
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--target-timeout-ms",
            "100",
        ],
    );

    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "target lifecycle exceeded its deadline by seconds"
    );
    assert_eq!(record["comparison"]["classification"], "target_error");
    assert!(record["candidate"]["error"]
        .as_str()
        .unwrap()
        .contains("timed out"));

    let pid = fs::read_to_string(&pid_path).unwrap();
    let mut alive = true;
    for _ in 0..20 {
        alive = std::process::Command::new("kill")
            .args(["-0", pid.trim()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !alive {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(!alive, "timed-out descendant should not survive Moonlight");

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
