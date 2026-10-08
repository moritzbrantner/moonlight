use crate::cli_support::{run_record, storage_path};
use assert_fs::TempDir;
use std::{fs, process::Command, thread, time::Duration};

#[test]
fn windows_timeout_terminates_descendants_even_after_parent_exits() {
    for parent_waits in [true, false] {
        let directory = TempDir::new().unwrap();
        let script = directory.path().join("spawn.ps1");
        let pid_file = directory.path().join("descendant.pid");
        fs::write(
            &script,
            format!(
                r#"
param([string] $PidFile)
$start = New-Object System.Diagnostics.ProcessStartInfo
$start.FileName = 'powershell.exe'
$start.Arguments = '-NoProfile -NonInteractive -Command Start-Sleep -Seconds 30'
$start.UseShellExecute = $false
$child = [System.Diagnostics.Process]::Start($start)
Set-Content -Path $PidFile -Value $child.Id
{}
"#,
                if parent_waits {
                    "Start-Sleep -Seconds 30"
                } else {
                    "exit 0"
                }
            ),
        )
        .unwrap();
        let primary = serde_json::to_string(&["cmd.exe", "/c", "echo ok"]).unwrap();
        let candidate = serde_json::to_string(&[
            "powershell.exe",
            "-NoProfile",
            "-NonInteractive",
            "-File",
            script.to_str().unwrap(),
            pid_file.to_str().unwrap(),
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
