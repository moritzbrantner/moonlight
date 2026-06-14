use assert_cmd::prelude::*;
use assert_fs::TempDir;
use serde_json::Value;
use std::{fs, path::Path, process::Command};

pub fn cli() -> Command {
    Command::cargo_bin("moonlight-cli").expect("moonlight-cli binary")
}

pub fn storage_path(dir: &TempDir) -> String {
    dir.path().join("runs.jsonl").to_string_lossy().into_owned()
}

pub fn json_command(value: &str) -> String {
    format!("printf '%s\\n' '{}'", value)
}

pub fn run_record(storage: &str, args: &[&str]) -> Value {
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

pub fn read_json(args: &[&str]) -> Value {
    let output = cli()
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("stdout should be JSON")
}

pub fn read_jsonl(path: &str) -> Vec<Value> {
    let content = fs::read_to_string(path).expect("storage should be readable");
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("storage line should be JSON"))
        .collect()
}

pub fn write_batch_cases(path: &Path, cases: &[Value]) {
    let lines = cases
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{lines}\n")).unwrap();
}
