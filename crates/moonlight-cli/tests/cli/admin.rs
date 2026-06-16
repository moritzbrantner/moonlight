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

    assert_eq!(runs["total"], 2);
    assert_eq!(runs["items"].as_array().unwrap().len(), 2);
    assert_eq!(runs["items"][0]["id"], second["id"]);
    assert_eq!(runs["items"][1]["id"], first["id"]);
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

    assert_eq!(runs["items"].as_array().unwrap().len(), 1);
    assert_eq!(runs["items"][0]["id"], record["id"]);
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

    assert_eq!(runs["items"].as_array().unwrap().len(), 1);
    assert_eq!(runs["items"][0]["id"], second["id"]);
    assert_ne!(runs["items"][0]["id"], third["id"]);
    assert_ne!(runs["items"][0]["id"], first["id"]);
    assert_eq!(runs["next_offset"], 2);
    assert!(!stdout.contains("\n  "));
    dir.close().unwrap();
}

#[test]
fn list_default_output_shape_stays_pretty_json_page() {
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

    assert!(stdout.starts_with("{\n"));
    assert!(stdout.contains("\"items\""));
    assert!(serde_json::from_str::<serde_json::Value>(&stdout)
        .unwrap()
        .is_object());
    dir.close().unwrap();
}

#[test]
fn list_supports_classification_adapter_query_and_status_filters() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let matching = run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":1}"#),
            "--candidate",
            &json_command(r#"{"value":2}"#),
        ],
    );
    run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":3}"#),
            "--candidate",
            &json_command(r#"{"value":3}"#),
        ],
    );

    let page = read_json(&[
        "list",
        "--storage-path",
        &storage,
        "--classification",
        "suspicious_difference",
        "--adapter",
        "cli",
        "--query",
        "value",
        "--status",
        "0",
    ]);

    assert_eq!(page["total"], 1);
    assert_eq!(page["items"][0]["id"], matching["id"]);
    dir.close().unwrap();
}

#[test]
fn report_renders_markdown_and_json() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let run = run_record(
        &storage,
        &[
            "--primary",
            &json_command(r#"{"value":1}"#),
            "--candidate",
            &json_command(r#"{"value":2}"#),
        ],
    );
    let id = run["id"].as_str().unwrap();

    cli()
        .args(["report", id, "--storage-path", &storage])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Moonlight Report"));
    let json = read_json(&["report", id, "--storage-path", &storage, "--format", "json"]);

    assert_eq!(json["run"]["id"], run["id"]);
    dir.close().unwrap();
}

#[test]
fn review_and_reviews_use_sidecar_state() {
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("moonlight.conf");
    let review_path = dir.path().join("review-state.json");
    fs::write(
        &config,
        format!(
            "[storage]\nreview_state_path = \"{}\"\n",
            review_path.display()
        ),
    )
    .unwrap();
    let id = Uuid::new_v4().to_string();

    let state = read_json(&[
        "--config",
        config.to_str().unwrap(),
        "review",
        &id,
        "--status",
        "ignored",
        "--note",
        "known",
        "--tag",
        "noise",
    ]);
    let reviews = read_json(&[
        "--config",
        config.to_str().unwrap(),
        "reviews",
        "--status",
        "ignored",
    ]);

    assert_eq!(state["status"], "ignored");
    assert_eq!(reviews.as_array().unwrap().len(), 1);
    assert_eq!(reviews[0]["run_id"], id);
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
fn cli_read_commands_use_config_storage_default() {
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let config = dir.path().join("moonlight.conf");
    fs::write(
        &config,
        format!("[storage]\npath = \"{}\"\n", storage.replace('\\', "\\\\")),
    )
    .unwrap();

    cli()
        .arg("--config")
        .arg(&config)
        .arg("run")
        .args([
            "--primary",
            &json_command(r#"{"value":42}"#),
            "--candidate",
            &json_command(r#"{"value":42}"#),
            "--quiet",
        ])
        .assert()
        .success();

    let stats = cli()
        .arg("--config")
        .arg(&config)
        .arg("stats")
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
fn explicit_storage_path_overrides_config_default() {
    let dir = TempDir::new().unwrap();
    let config_storage = dir
        .path()
        .join("config-runs.jsonl")
        .to_string_lossy()
        .into_owned();
    let explicit_storage = dir
        .path()
        .join("explicit-runs.jsonl")
        .to_string_lossy()
        .into_owned();
    let config = dir.path().join("moonlight.conf");
    fs::write(
        &config,
        format!(
            "[storage]\npath = \"{}\"\n",
            config_storage.replace('\\', "\\\\")
        ),
    )
    .unwrap();
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
        .arg("--config")
        .arg(&config)
        .args(["stats", "--storage-path", &explicit_storage])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stats: serde_json::Value = serde_json::from_slice(&stats).unwrap();

    assert_eq!(stats["total_runs"], 1);
    assert!(!Path::new(&config_storage).exists());
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
    assert_eq!(list["items"].as_array().unwrap().len(), 2);
    assert_eq!(shown["id"], second["id"]);
    dir.close().unwrap();
}

#[allow(dead_code)]
fn _assert_path_is_outside_repo(path: &Path) {
    assert!(path.is_absolute());
}
