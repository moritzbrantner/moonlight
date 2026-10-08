use crate::cli_support::{run_record, storage_path};
use assert_fs::TempDir;
use std::{fs, process::Command, thread, time::Duration};

#[test]
fn windows_timeout_terminates_descendants_even_after_parent_exits() {
    let fixture_directory = TempDir::new().unwrap();
    let source = fixture_directory.path().join("descendant.rs");
    let executable = fixture_directory.path().join("descendant.exe");
    fs::write(
        &source,
        r#"
use std::{env, fs, process::Command, thread, time::Duration};
fn main() {
    let args: Vec<_> = env::args().collect();
    if args[1] == "child" {
        thread::sleep(Duration::from_secs(30));
        return;
    }
    let child = Command::new(env::current_exe().unwrap()).arg("child").spawn().unwrap();
    fs::write(&args[1], child.id().to_string()).unwrap();
    if args[2] == "wait" { thread::sleep(Duration::from_secs(30)); }
}
"#,
    )
    .unwrap();
    let compiled = Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "fixture compilation failed: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    for parent_waits in [true, false] {
        let directory = TempDir::new().unwrap();
        let pid_file = directory.path().join("descendant.pid");
        let primary = serde_json::to_string(&["cmd.exe", "/c", "echo ok"]).unwrap();
        let candidate = serde_json::to_string(&[
            executable.to_str().unwrap(),
            pid_file.to_str().unwrap(),
            if parent_waits { "wait" } else { "exit" },
        ])
        .unwrap();
        let record = run_record(
            &storage_path(&directory),
            &[
                "--primary-argv",
                &primary,
                "--candidate-argv",
                &candidate,
                "--target-timeout-ms",
                "4000",
            ],
        );
        let pid: u32 = fs::read_to_string(&pid_file)
            .expect("descendant started")
            .trim()
            .parse()
            .unwrap();
        let probe = format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 1 }} else {{ exit 0 }}");
        let mut gone = false;
        for _ in 0..20 {
            gone = Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", &probe])
                .status()
                .unwrap()
                .success();
            if gone {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        if !gone {
            // Clean up this test-owned process even when the regression fails.
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status();
        }
        assert!(gone, "descendant survived; parent_waits={parent_waits}");
        assert_eq!(record["comparison"]["classification"], "target_error");
        assert!(record["candidate"]["error"]
            .as_str()
            .unwrap()
            .contains("timed out"));
    }
}
