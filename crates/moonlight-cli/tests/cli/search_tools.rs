use crate::cli_support::{read_json, read_jsonl, storage_path, write_batch_cases};
use assert_fs::TempDir;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct NamedCase {
    name: &'static str,
    case: Value,
}

struct SearchFixture {
    _dir: TempDir,
    root: PathBuf,
    readme: String,
    main_rs: String,
    lib_rs: String,
    notes: String,
}

impl SearchFixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let src = root.join("src");
        let docs = root.join("docs");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&docs).unwrap();

        let readme = root.join("README.txt");
        let main_rs = src.join("main.rs");
        let lib_rs = src.join("lib.rs");
        let notes = docs.join("notes.txt");

        fs::write(
            &readme,
            [
                "Alpha moon",
                "beta Moonlight",
                "literal.*value",
                "numbers 12345",
                "target: one",
                "skip this",
            ]
            .join("\n")
                + "\n",
        )
        .unwrap();
        fs::write(
            &main_rs,
            [
                r#"fn main() { println!("moon"); }"#,
                "let target = 2;",
                "let word_boundary = moon;",
            ]
            .join("\n")
                + "\n",
        )
        .unwrap();
        fs::write(
            &lib_rs,
            [
                "pub fn helper() -> &'static str {",
                r#"    "Moonlight helper""#,
                "}",
            ]
            .join("\n")
                + "\n",
        )
        .unwrap();
        fs::write(&notes, "needle visible\nmoon notes\n").unwrap();
        fs::write(root.join(".hidden"), "needle hidden\n").unwrap();
        fs::write(root.join(".ignore"), "ignored.log\n").unwrap();
        fs::write(root.join("ignored.log"), "needle ignored\n").unwrap();

        Self {
            _dir: dir,
            root,
            readme: path_string(&readme),
            main_rs: path_string(&main_rs),
            lib_rs: path_string(&lib_rs),
            notes: path_string(&notes),
        }
    }
}

#[test]
fn batch_compares_grep_and_ripgrep_equivalence_matrix() {
    if !commands_available(&["grep", "rg"]) {
        return;
    }
    let fixture = SearchFixture::new();
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    let cases = grep_rg_equivalence_cases(&fixture);
    write_batch_cases(
        &input_path,
        &cases
            .iter()
            .map(|case| case.case.clone())
            .collect::<Vec<_>>(),
    );

    let summary = read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);
    let records = read_jsonl(&storage);

    assert_eq!(summary["total_runs"], cases.len());
    assert_eq!(summary["matches"], cases.len());
    assert_eq!(summary["suspicious_differences"], 0);
    assert_eq!(summary["target_errors"], 0);
    assert_eq!(records.len(), cases.len());

    for (record, case) in records.iter().zip(cases.iter()) {
        assert_eq!(
            record["comparison"]["classification"], "match",
            "{} should match",
            case.name
        );
        assert_eq!(
            record["primary"]["status"], record["candidate"]["status"],
            "{} should preserve grep/rg exit status",
            case.name
        );
        assert_eq!(
            record["primary"]["body"]["sha256"], record["candidate"]["body"]["sha256"],
            "{} should capture identical stdout",
            case.name
        );
        assert_eq!(
            record["primary"]["stderr"]["sha256"], record["candidate"]["stderr"]["sha256"],
            "{} should capture identical stderr",
            case.name
        );
    }

    dir.close().unwrap();
}

#[test]
fn batch_reports_known_grep_and_ripgrep_behavior_differences() {
    if !commands_available(&["grep", "rg"]) {
        return;
    }
    let fixture = SearchFixture::new();
    let dir = TempDir::new().unwrap();
    let storage = storage_path(&dir);
    let input_path = dir.path().join("cases.jsonl");
    let cases = grep_rg_difference_cases(&fixture);
    write_batch_cases(
        &input_path,
        &cases
            .iter()
            .map(|case| case.case.clone())
            .collect::<Vec<_>>(),
    );

    let summary = read_json(&[
        "batch",
        "--input",
        input_path.to_str().unwrap(),
        "--storage-path",
        &storage,
        "--jobs",
        "1",
    ]);
    let records = read_jsonl(&storage);

    assert_eq!(summary["total_runs"], cases.len());
    assert_eq!(summary["matches"], 0);
    assert_eq!(summary["suspicious_differences"], cases.len());
    assert_eq!(summary["target_errors"], 0);

    for (record, case) in records.iter().zip(cases.iter()) {
        assert_eq!(
            record["comparison"]["classification"], "suspicious_difference",
            "{} should be reported as a real behavior difference",
            case.name
        );
        assert!(
            !record["comparison"]["noise_filtered_diffs"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{} should include actionable diff entries",
            case.name
        );
    }

    assert_diff_kind(
        &records[0],
        "body",
        "default recursion should expose stdout drift",
    );
    assert_diff_kind(
        &records[1],
        "body",
        "count output should expose stdout drift",
    );
    assert_diff_kind(
        &records[2],
        "stderr",
        "invalid regex should expose stderr drift",
    );

    dir.close().unwrap();
}

fn grep_rg_equivalence_cases(fixture: &SearchFixture) -> Vec<NamedCase> {
    let root = path_string(&fixture.root);
    vec![
        named(
            "single-file basic line match",
            argv_case(
                &["grep", "-n", "--", "Moonlight", &fixture.readme],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "--",
                    "Moonlight",
                    &fixture.readme,
                ],
            ),
        ),
        named(
            "case-insensitive multi-file match",
            argv_case(
                &[
                    "grep",
                    "-n",
                    "-i",
                    "--",
                    "moon",
                    &fixture.main_rs,
                    &fixture.readme,
                ],
                &[
                    "rg",
                    "--sort",
                    "path",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "-i",
                    "--",
                    "moon",
                    &fixture.main_rs,
                    &fixture.readme,
                ],
            ),
        ),
        named(
            "fixed-string metacharacters",
            argv_case(
                &["grep", "-n", "-F", "--", "literal.*value", &fixture.readme],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "--fixed-strings",
                    "--",
                    "literal.*value",
                    &fixture.readme,
                ],
            ),
        ),
        named(
            "extended alternation regex",
            argv_case(
                &["grep", "-n", "-E", "--", "moon|target", &fixture.main_rs],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "--",
                    "moon|target",
                    &fixture.main_rs,
                ],
            ),
        ),
        named(
            "word-regexp match",
            argv_case(
                &["grep", "-n", "-w", "--", "moon", &fixture.main_rs],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "-w",
                    "--",
                    "moon",
                    &fixture.main_rs,
                ],
            ),
        ),
        named(
            "only-matching numeric regex",
            argv_case(
                &["grep", "-n", "-o", "-E", "--", "[0-9]+", &fixture.readme],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "-o",
                    "--",
                    "[0-9]+",
                    &fixture.readme,
                ],
            ),
        ),
        named(
            "inverted single-file match",
            argv_case(
                &["grep", "-n", "-v", "--", "skip", &fixture.readme],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "-v",
                    "--",
                    "skip",
                    &fixture.readme,
                ],
            ),
        ),
        named(
            "context formatting",
            argv_case(
                &["grep", "-n", "-C", "1", "--", "target", &fixture.readme],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "-C",
                    "1",
                    "--",
                    "target",
                    &fixture.readme,
                ],
            ),
        ),
        named(
            "files-with-matches output",
            argv_case(
                &[
                    "grep",
                    "-l",
                    "--",
                    "moon",
                    &fixture.main_rs,
                    &fixture.readme,
                    &fixture.notes,
                ],
                &[
                    "rg",
                    "--sort",
                    "path",
                    "-l",
                    "--color",
                    "never",
                    "--",
                    "moon",
                    &fixture.main_rs,
                    &fixture.readme,
                    &fixture.notes,
                ],
            ),
        ),
        named(
            "quiet matching exit status",
            argv_case(
                &["grep", "-q", "--", "Moonlight", &fixture.lib_rs],
                &[
                    "rg",
                    "-q",
                    "--color",
                    "never",
                    "--",
                    "Moonlight",
                    &fixture.lib_rs,
                ],
            ),
        ),
        named(
            "quiet no-match exit status",
            argv_case(
                &["grep", "-q", "--", "absent", &fixture.lib_rs],
                &[
                    "rg",
                    "-q",
                    "--color",
                    "never",
                    "--",
                    "absent",
                    &fixture.lib_rs,
                ],
            ),
        ),
        named(
            "first-match limit",
            argv_case(
                &["grep", "-n", "-m", "1", "--", "moon", &fixture.readme],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "-m",
                    "1",
                    "--",
                    "moon",
                    &fixture.readme,
                ],
            ),
        ),
        named(
            "recursive search with ignore behavior disabled",
            json!({
                "primary": format!("grep -R -n -- needle {} | sort", shell_quote(&root)),
                "candidate_argv": [
                    "rg",
                    "-uu",
                    "--sort",
                    "path",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "--",
                    "needle",
                    root,
                ],
            }),
        ),
    ]
}

fn grep_rg_difference_cases(fixture: &SearchFixture) -> Vec<NamedCase> {
    let root = path_string(&fixture.root);
    vec![
        named(
            "ripgrep default skips hidden and ignored files",
            json!({
                "primary": format!("grep -R -n -- needle {} | sort", shell_quote(&root)),
                "candidate_argv": [
                    "rg",
                    "--sort",
                    "path",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "--",
                    "needle",
                    root,
                ],
            }),
        ),
        named(
            "count mode no-match reporting",
            argv_case(
                &[
                    "grep",
                    "-c",
                    "--",
                    "absent",
                    &fixture.main_rs,
                    &fixture.readme,
                ],
                &[
                    "rg",
                    "--sort",
                    "path",
                    "-c",
                    "--no-heading",
                    "--color",
                    "never",
                    "--",
                    "absent",
                    &fixture.main_rs,
                    &fixture.readme,
                ],
            ),
        ),
        named(
            "invalid regex diagnostics",
            argv_case(
                &["grep", "-n", "--", "[", &fixture.readme],
                &[
                    "rg",
                    "-n",
                    "--no-heading",
                    "--color",
                    "never",
                    "--",
                    "[",
                    &fixture.readme,
                ],
            ),
        ),
    ]
}

fn named(name: &'static str, case: Value) -> NamedCase {
    NamedCase { name, case }
}

fn argv_case(primary: &[&str], candidate: &[&str]) -> Value {
    json!({
        "primary_argv": primary,
        "candidate_argv": candidate,
    })
}

fn assert_diff_kind(record: &Value, kind: &str, message: &str) {
    assert!(
        record["comparison"]["noise_filtered_diffs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diff| diff["kind"] == kind),
        "{message}"
    );
}

fn commands_available(commands: &[&str]) -> bool {
    commands
        .iter()
        .all(|command| Command::new(command).arg("--help").output().is_ok())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
