use assert_cmd::cargo::cargo_bin;
use chrono::Utc;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use moonlight_core::{
    Adapter, BodyCapture, Classification, ComparisonRun, ComparisonSummary, RunInput,
    TargetObservation,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::{tempdir, TempDir};
use uuid::Uuid;

fn binary() -> PathBuf {
    cargo_bin("moonlight-cli")
}

fn command_json(value: &str) -> String {
    format!("printf '%s\\n' '{}'", value)
}

fn fresh_storage() -> (TempDir, PathBuf) {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("runs.jsonl");
    (dir, path)
}

fn run_cli(args: &[&str]) {
    let output = Command::new(binary())
        .args(args)
        .output()
        .expect("run moonlight-cli");
    assert!(
        output.status.success(),
        "moonlight-cli failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_scenario(storage_path: &Path, args: &[&str]) {
    let storage = storage_path.to_string_lossy();
    let mut full_args = vec!["run", "--storage-path", storage.as_ref()];
    full_args.extend_from_slice(args);
    run_cli(&full_args);
}

fn body() -> BodyCapture {
    BodyCapture {
        size_bytes: 0,
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        preview: String::new(),
        truncated: false,
    }
}

fn target(latency_ms: u128) -> TargetObservation {
    TargetObservation {
        status: Some(0),
        headers: BTreeMap::new(),
        body: body(),
        stderr: Some(body()),
        latency_ms,
        error: None,
    }
}

fn fixture_run(index: usize) -> ComparisonRun {
    ComparisonRun {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        adapter: Adapter::Cli,
        input: RunInput::Cli {
            primary_command: format!("fixture-{index}-primary"),
            candidate_command: format!("fixture-{index}-candidate"),
            secondary_command: Some(format!("fixture-{index}-secondary")),
        },
        request_headers: BTreeMap::new(),
        request_body: body(),
        primary: target(10),
        candidate: target(20),
        secondary: Some(target(30)),
        comparison: ComparisonSummary {
            classification: if index.is_multiple_of(2) {
                Classification::Match
            } else {
                Classification::SuspiciousDifference
            },
            ..Default::default()
        },
    }
}

fn write_fixture(count: usize) -> (TempDir, PathBuf, Uuid) {
    let (dir, path) = fresh_storage();
    let mut show_id = Uuid::nil();
    let lines = (0..count)
        .map(|index| {
            let run = fixture_run(index);
            if index == count / 2 {
                show_id = run.id;
            }
            serde_json::to_string(&run).expect("serialize fixture run")
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&path, format!("{lines}\n")).expect("write fixture");
    (dir, path, show_id)
}

fn bench_run_commands(c: &mut Criterion) {
    c.bench_function("run_match_small_json", |b| {
        let primary = command_json(r#"{"value":42}"#);
        let candidate = command_json(r#"{"value":42}"#);
        let secondary = command_json(r#"{"value":42}"#);
        b.iter_batched(
            fresh_storage,
            |(_dir, storage)| {
                run_scenario(
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
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("run_suspicious_difference_small_json", |b| {
        let primary = command_json(r#"{"value":42}"#);
        let candidate = command_json(r#"{"value":43}"#);
        b.iter_batched(
            fresh_storage,
            |(_dir, storage)| {
                run_scenario(
                    &storage,
                    &["--primary", &primary, "--candidate", &candidate],
                );
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("run_reference_noise_small_json", |b| {
        let primary = command_json(r#"{"region":"a","value":1}"#);
        let candidate = command_json(r#"{"region":"a","value":1}"#);
        let secondary = command_json(r#"{"region":"b","value":1}"#);
        b.iter_batched(
            fresh_storage,
            |(_dir, storage)| {
                run_scenario(
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
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("run_large_stdout_64kb", |b| {
        let primary = "python3 -c 'print(\"a\" * 65536, end=\"\")'";
        let candidate = "python3 -c 'print(\"a\" * 65536, end=\"\")'";
        b.iter_batched(
            fresh_storage,
            |(_dir, storage)| {
                run_scenario(&storage, &["--primary", primary, "--candidate", candidate]);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_read_commands(c: &mut Criterion) {
    let (_dir, storage, show_id) = write_fixture(1000);
    let storage = storage.to_string_lossy().into_owned();
    let show_id = show_id.to_string();

    c.bench_function("stats_1000_runs", |b| {
        b.iter(|| run_cli(&["stats", "--storage-path", &storage]));
    });

    c.bench_function("list_1000_runs", |b| {
        b.iter(|| run_cli(&["list", "--storage-path", &storage]));
    });

    c.bench_function("show_middle_run_1000_runs", |b| {
        b.iter(|| run_cli(&["show", &show_id, "--storage-path", &storage]));
    });
}

criterion_group!(benches, bench_run_commands, bench_read_commands);
criterion_main!(benches);
