use crate::cli_support::{cli, json_command, read_json, run_record, storage_path};
use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use std::{fs, path::Path};
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
fn list_alias_lists_runs() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let record = run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":1}"#),
            "--candidate",
            &json_command(r#"{"value":1}"#),
        ],
    );

    let runs = read_json(&["ls", "--storage-path", &storage]);

    assert_eq!(runs.as_array().unwrap().len(), 1);
    assert_eq!(runs[0]["id"], record["id"]);
    dir.close().unwrap();
}

#[test]
fn list_supports_limit_offset_and_compact_output() {
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
    let third = run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":3}"#),
            "--candidate",
            &json_command(r#"{"value":3}"#),
        ],
    );

    let output = cli()
        .args([
            "list",
            "--storage-path",
            &storage,
            "--limit",
            "1",
            "--offset",
            "1",
            "--compact",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let runs: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(runs.as_array().unwrap().len(), 1);
    assert_eq!(runs[0]["id"], second["id"]);
    assert_ne!(runs[0]["id"], third["id"]);
    assert_ne!(runs[0]["id"], first["id"]);
    assert!(!stdout.contains("\n  "));
    dir.close().unwrap();
}

#[test]
fn list_default_output_shape_stays_pretty_json_array() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":1}"#),
            "--candidate",
            &json_command(r#"{"value":1}"#),
        ],
    );

    let output = cli()
        .args(["list", "--storage-path", &storage])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(stdout.starts_with("[\n"));
    assert!(stdout.contains("\n  {"));
    assert!(serde_json::from_str::<serde_json::Value>(&stdout)
        .unwrap()
        .is_array());
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
fn stats_supports_compact_output() {
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

    let output = cli()
        .args(["stats", "--storage-path", &storage, "--compact"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let stats: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(stats["total_runs"], 1);
    assert!(!stdout.contains("\n  "));
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
fn show_supports_compact_output() {
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

    let output = cli()
        .args(["show", id, "--storage-path", &storage, "--compact"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let shown: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(shown["id"], record["id"]);
    assert!(!stdout.contains("\n  "));
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

#[test]
fn cli_read_commands_use_env_storage_default() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);

    cli()
        .arg("run")
        .args([
            "--primary",
            &json_command(r#"{"value":42}"#),
            "--candidate",
            &json_command(r#"{"value":42}"#),
            "--quiet",
        ])
        .env("MOONLIGHT_CLI_STORAGE_PATH", &storage)
        .assert()
        .success();

    let stats = cli()
        .arg("stats")
        .env("MOONLIGHT_CLI_STORAGE_PATH", &storage)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stats: serde_json::Value = serde_json::from_slice(&stats).unwrap();

    assert_eq!(stats["total_runs"], 1);
    dir.close().unwrap();
}

#[test]
fn explicit_storage_path_overrides_env_default() {
    let dir = TempDir::new().unwrap();
    let env_storage = dir
        .path()
        .join("env-runs.jsonl")
        .to_string_lossy()
        .into_owned();
    let explicit_storage = dir
        .path()
        .join("explicit-runs.jsonl")
        .to_string_lossy()
        .into_owned();
    run_record(
        &explicit_storage,
        &[
            "--primary",
            &json_command(r#"{"value":42}"#),
            "--candidate",
            &json_command(r#"{"value":42}"#),
        ],
    );

    let stats = cli()
        .args(["stats", "--storage-path", &explicit_storage])
        .env("MOONLIGHT_CLI_STORAGE_PATH", &env_storage)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stats: serde_json::Value = serde_json::from_slice(&stats).unwrap();

    assert_eq!(stats["total_runs"], 1);
    assert!(!Path::new(&env_storage).exists());
    dir.close().unwrap();
}

#[test]
fn cli_read_commands_ignore_sibling_jsonl_files() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let sibling = dir.path().join("sibling.jsonl");
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
    fs::write(&sibling, format!("{}\n", first)).unwrap();

    let stats = read_json(&["stats", "--storage-path", &storage]);
    let list = read_json(&["list", "--storage-path", &storage]);
    let shown = read_json(&[
        "show",
        second["id"].as_str().unwrap(),
        "--storage-path",
        &storage,
    ]);

    assert_eq!(stats["total_runs"], 2);
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(shown["id"], second["id"]);
    dir.close().unwrap();
}

#[allow(dead_code)]
fn _assert_path_is_outside_repo(path: &Path) {
    assert!(path.is_absolute());
}
