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
    fs::write(&candidate_path, r#"{"visible":1,"token":"AUDIT_SENTINEL"}"#).unwrap();

    let primary = serde_json::to_string(&["cat", primary_path.to_str().unwrap()]).unwrap();
    let candidate = serde_json::to_string(&["cat", candidate_path.to_str().unwrap()]).unwrap();

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
    let primary = serde_json::to_string(&["printf", "%s", "ok"]).unwrap();
    let candidate = serde_json::to_string(&[
        "sh",
        "-c",
        "sleep 30 & echo $! > \"$1\"",
        "fixture",
        pid_path.to_str().unwrap(),
    ])
    .unwrap();

    let started = std::time::Instant::now();
    let record = run_record(
        &storage,
        &[
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--target-timeout-ms",
            "500",
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

#[cfg(unix)]
#[test]
fn interrupt_stops_isolated_target_processes() {
    for (signal, exit_code) in [("-INT", 130), ("-TERM", 143), ("-HUP", 129), ("-QUIT", 131)] {
        assert_signal_cleans_targets(signal, exit_code);
    }
}

#[cfg(unix)]
fn assert_signal_cleans_targets(signal: &str, exit_code: i32) {
    use std::os::unix::process::CommandExt;
    use std::{
        process::{Command, Stdio},
        thread,
        time::Duration,
    };
    let dir = TempDir::new().unwrap();
    let pid_path = dir.path().join("interrupted-targets.pid");
    let candidate = serde_json::to_string(&[
        "sh",
        "-c",
        "echo $$ > \"$1\"; sleep 30 & echo $! >> \"$1\"; wait",
        "fixture",
        pid_path.to_str().unwrap(),
    ])
    .unwrap();
    let primary = serde_json::to_string(&["printf", "%s", "ok"]).unwrap();
    let mut child = crate::cli_support::cli()
        .args([
            "run",
            "--storage-path",
            &storage_path(&dir),
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--target-timeout-ms",
            "30000",
        ])
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut pids = Vec::<String>::new();
    for _ in 0..100 {
        if let Ok(text) = fs::read_to_string(&pid_path) {
            pids = text.lines().map(str::to_owned).collect();
        }
        if pids.len() == 2 {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    Command::new("kill")
        .args([signal, "--", &format!("-{}", child.id())])
        .status()
        .unwrap();
    let status = child.wait().unwrap();
    let mut survivors = Vec::new();
    for pid in &pids {
        let mut alive = true;
        for _ in 0..20 {
            alive = Command::new("kill")
                .args(["-0", pid])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success();
            if !alive {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        if alive {
            survivors.push(pid.clone());
        }
    }
    for pid in &survivors {
        let _ = Command::new("kill").args(["-KILL", pid]).status();
    }
    assert_eq!(pids.len(), 2, "target and descendant must have started");
    assert!(
        survivors.is_empty(),
        "{signal} left target processes running: {survivors:?}"
    );
    assert_eq!(status.code(), Some(exit_code));
}

#[cfg(target_os = "linux")]
#[test]
fn timeout_cleans_descendants_that_create_new_sessions() {
    assert_linux_background_cleanup("setsid sleep 30 & echo $! > \"$1\"; wait", true);
}

#[cfg(target_os = "linux")]
#[test]
fn successful_commands_clean_redirected_background_children() {
    assert_linux_background_cleanup(
        "sleep 30 </dev/null >/dev/null 2>&1 & echo $! > \"$1\"",
        false,
    );
}

#[cfg(target_os = "linux")]
fn assert_linux_background_cleanup(script: &str, times_out: bool) {
    let dir = TempDir::new().unwrap();
    let pid_path = dir.path().join("background.pid");
    let primary = serde_json::to_string(&["true"]).unwrap();
    let candidate =
        serde_json::to_string(&["sh", "-c", script, "fixture", pid_path.to_str().unwrap()])
            .unwrap();
    let record = run_record(
        &storage_path(&dir),
        &[
            "--primary-argv",
            &primary,
            "--candidate-argv",
            &candidate,
            "--target-timeout-ms",
            "500",
        ],
    );
    let pid = fs::read_to_string(&pid_path).unwrap();
    let alive = process_exists(pid.trim());
    if alive {
        let _ = std::process::Command::new("kill")
            .args(["-KILL", pid.trim()])
            .status();
    }
    assert_eq!(
        record["comparison"]["classification"],
        if times_out { "target_error" } else { "match" }
    );
    assert!(!alive, "background child survived target completion");
}

#[cfg(target_os = "linux")]
fn process_exists(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
        .success()
}

#[cfg(target_os = "linux")]
#[test]
fn batch_reaps_timed_out_descendants_before_the_next_case_finishes() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("cases.jsonl");
    let pid_path = dir.path().join("first.pid");
    let second_started = dir.path().join("second.started");
    crate::cli_support::write_batch_cases(
        &input,
        &[
            serde_json::json!({"primary_argv": ["true"], "candidate_argv": ["sh", "-c", "sleep 30 & echo $! > \"$1\"; wait", "fixture", pid_path], "target_timeout_ms": 500}),
            serde_json::json!({"primary_argv": ["true"], "candidate_argv": ["sh", "-c", "touch \"$1\"; sleep 3", "fixture", second_started], "target_timeout_ms": 5000}),
        ],
    );
    let mut child = crate::cli_support::cli()
        .args([
            "batch",
            "--input",
            input.to_str().unwrap(),
            "--storage-path",
            &storage_path(&dir),
            "--jobs",
            "1",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..150 {
        if second_started.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let pid = fs::read_to_string(&pid_path).unwrap();
    let alive_during_second = process_exists(pid.trim());
    let status = child.wait().unwrap();
    assert!(second_started.exists());
    assert!(status.success());
    assert!(
        !alive_during_second,
        "first descendant remained a zombie during the next case"
    );
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[test]
fn missing_optional_proc_children_preserves_success_and_descendant_cleanup() {
    use std::process::{Command, Stdio};
    let directory = TempDir::new().unwrap();
    let source = directory.path().join("without-children.c");
    let library = directory.path().join("without-children.so");
    fs::write(
        &source,
        r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <string.h>
#include <sys/types.h>
static int call_open(const char *symbol, const char *path, int flags, mode_t mode) {
    const size_t size = strlen(path);
    if (strstr(path, "/proc/self/task/") && size >= 9 && !strcmp(path + size - 9, "/children")) {
        errno = ENOENT;
        return -1;
    }
    int (*real_open)(const char *, int, ...) = dlsym(RTLD_NEXT, symbol);
    return real_open(path, flags, mode);
}
int open(const char *path, int flags, ...) {
    va_list args; va_start(args, flags);
    mode_t mode = flags & O_CREAT ? va_arg(args, int) : 0;
    va_end(args);
    return call_open("open", path, flags, mode);
}
int open64(const char *path, int flags, ...) {
    va_list args; va_start(args, flags);
    mode_t mode = flags & O_CREAT ? va_arg(args, int) : 0;
    va_end(args);
    return call_open("open64", path, flags, mode);
}
"#,
    )
    .unwrap();
    let compiler = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(&source)
        .args(["-o"])
        .arg(&library)
        .arg("-ldl")
        .output()
        .unwrap();
    assert!(
        compiler.status.success(),
        "{}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    let pid_path = directory.path().join("descendant.pid");
    let primary = json_command(r#"{"value":1}"#);
    let timeout_command = format!("sleep 30 & echo $! > '{}'; wait", pid_path.display());
    for (candidate, classification) in [(&primary, "match"), (&timeout_command, "target_error")] {
        let output = crate::cli_support::cli()
            .env("LD_PRELOAD", &library)
            .args([
                "run",
                "--storage-path",
                &storage_path(&directory),
                "--primary",
                &primary,
                "--candidate",
                candidate,
                "--target-timeout-ms",
                "500",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let record: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            record["comparison"]["classification"], classification,
            "{record}"
        );
    }
    let pid = fs::read_to_string(pid_path).unwrap();
    let alive = Command::new("kill")
        .args(["-0", pid.trim()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    if alive {
        let _ = Command::new("kill").args(["-KILL", pid.trim()]).status();
    }
    assert!(!alive, "descendant survived fallback cleanup");
}
