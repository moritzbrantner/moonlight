use crate::cli_support::{cli, read_jsonl, storage_path};
use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn eval_candidate_ref_match_exits_success_and_stores_project_run() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path(), "ok\n");
    let storage = storage_path(&dir);
    let config = write_eval_config(&repo, "cat value.txt", "never");

    let output = cli()
        .args([
            "eval",
            "run",
            "--project",
            config.to_str().unwrap(),
            "--candidate-ref",
            "main",
            "--storage-path",
            &storage,
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&output).unwrap();
    let records = read_jsonl(&storage);

    assert_eq!(summary["total_checks"], 1);
    assert_eq!(summary["failed_checks"].as_array().unwrap().len(), 0);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["adapter"], "project");
    assert_eq!(records[0]["input"]["project"], "eval-demo");
    assert_eq!(records[0]["comparison"]["classification"], "match");

    cli()
        .args([
            "eval",
            "report",
            summary["eval_id"].as_str().unwrap(),
            "--storage-path",
            &storage,
            "--format",
            "markdown",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Moonlight Eval"));
    dir.close().unwrap();
}

#[test]
fn eval_candidate_ref_failure_exits_one_and_reports_failed_check() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path(), "ok\n");
    commit_value(&repo, "candidate", "bad\n");
    let storage = storage_path(&dir);
    let config = write_eval_config(&repo, "grep ok value.txt", "failed");

    let output = cli()
        .args([
            "eval",
            "run",
            "--project",
            config.to_str().unwrap(),
            "--candidate-ref",
            "candidate",
            "--storage-path",
            &storage,
            "--format",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&output).unwrap();
    let records = read_jsonl(&storage);

    assert_eq!(summary["failed_checks"][0]["check_id"], "test");
    assert_eq!(
        summary["failed_checks"][0]["classification"],
        "suspicious_difference"
    );
    assert_eq!(
        records[0]["comparison"]["classification"],
        "suspicious_difference"
    );
    dir.close().unwrap();
}

#[test]
fn eval_candidate_patch_stdout_diff_is_visible_in_text_summary() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path(), "ok\n");
    let patch = write_value_patch(&repo, "bad\n");
    let storage = storage_path(&dir);
    let config = write_eval_config(&repo, "cat value.txt", "failed");

    cli()
        .args([
            "eval",
            "run",
            "--project",
            config.to_str().unwrap(),
            "--candidate-patch",
            patch.to_str().unwrap(),
            "--storage-path",
            &storage,
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("FAIL test").and(predicate::str::contains("diff:")));

    let records = read_jsonl(&storage);
    assert_eq!(
        records[0]["input"]["candidate_source"],
        format!("patch {}", patch.display())
    );
    dir.close().unwrap();
}

#[test]
fn eval_patch_apply_failure_exits_two_and_stores_no_runs() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path(), "ok\n");
    let storage = storage_path(&dir);
    let config = write_eval_config(&repo, "cat value.txt", "failed");
    let patch = dir.path().join("bad.patch");
    fs::write(&patch, "not a patch\n").unwrap();

    cli()
        .args([
            "eval",
            "run",
            "--project",
            config.to_str().unwrap(),
            "--candidate-patch",
            patch.to_str().unwrap(),
            "--storage-path",
            &storage,
            "--quiet",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to apply patch"));

    assert!(!Path::new(&storage).exists());
    dir.close().unwrap();
}

#[test]
fn eval_keep_worktrees_never_removes_successful_worktrees() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path(), "ok\n");
    let storage = storage_path(&dir);
    let config = write_eval_config(&repo, "cat value.txt", "never");

    let output = cli()
        .args([
            "eval",
            "run",
            "--project",
            config.to_str().unwrap(),
            "--candidate-ref",
            "main",
            "--storage-path",
            &storage,
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&output).unwrap();
    let eval_dir = repo
        .join(".moonlight/evals")
        .join(summary["eval_id"].as_str().unwrap());

    assert!(!eval_dir.exists());
    dir.close().unwrap();
}

#[test]
fn eval_keep_worktrees_failed_keeps_failed_worktrees() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path(), "ok\n");
    commit_value(&repo, "candidate", "bad\n");
    let storage = storage_path(&dir);
    let config = write_eval_config(&repo, "grep ok value.txt", "failed");

    let output = cli()
        .args([
            "eval",
            "run",
            "--project",
            config.to_str().unwrap(),
            "--candidate-ref",
            "candidate",
            "--storage-path",
            &storage,
            "--format",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&output).unwrap();
    let eval_dir = repo
        .join(".moonlight/evals")
        .join(summary["eval_id"].as_str().unwrap());

    assert!(eval_dir.join("primary").exists());
    assert!(eval_dir.join("candidate").exists());
    dir.close().unwrap();
}

fn init_repo(root: &Path, value: &str) -> PathBuf {
    let repo = root.join("repo");
    fs::create_dir(&repo).unwrap();
    git(root, ["init", "-b", "main", repo.to_str().unwrap()]);
    git(&repo, ["config", "user.email", "moonlight@example.com"]);
    git(&repo, ["config", "user.name", "Moonlight"]);
    fs::write(repo.join("value.txt"), value).unwrap();
    git(&repo, ["add", "value.txt"]);
    git(&repo, ["commit", "-m", "initial"]);
    repo
}

fn commit_value(repo: &Path, branch: &str, value: &str) {
    git(repo, ["checkout", "-b", branch]);
    fs::write(repo.join("value.txt"), value).unwrap();
    git(repo, ["add", "value.txt"]);
    git(repo, ["commit", "-m", "candidate"]);
    git(repo, ["checkout", "main"]);
}

fn write_value_patch(repo: &Path, value: &str) -> PathBuf {
    fs::write(repo.join("value.txt"), value).unwrap();
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("diff")
        .arg("--binary")
        .arg("main")
        .output()
        .unwrap();
    assert!(output.status.success());
    fs::write(repo.join("value.txt"), "ok\n").unwrap();
    let patch = repo.join("agent.patch");
    fs::write(&patch, output.stdout).unwrap();
    patch
}

fn write_eval_config(repo: &Path, command: &str, keep_worktrees: &str) -> PathBuf {
    let config = repo.join("moonlight.eval.toml");
    fs::write(
        &config,
        format!(
            r#"
[project]
name = "eval-demo"
repo = "{}"
baseline_ref = "main"

[eval]
work_dir = ".moonlight/evals"
keep_worktrees = "{keep_worktrees}"
jobs = 1
target_timeout_ms = 120000
max_body_capture_bytes = 20000

[[checks]]
id = "test"
command = "{}"
cwd = "."
"#,
            repo.display(),
            command.replace('"', "\\\"")
        ),
    )
    .unwrap();
    config
}

fn git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
