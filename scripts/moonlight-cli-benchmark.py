#!/usr/bin/env python3
import argparse
import concurrent.futures
import datetime as dt
import glob
import json
import os
import shutil
import shlex
import statistics
import subprocess
import time
import uuid
from pathlib import Path


DEFAULT_TARGETS = ["moonlight", "moonlight-argv", "trycmd", "insta", "cram", "bats", "shellspec"]
DEFAULT_SCENARIOS = [
    "match",
    "match-argv",
    "candidate-diff",
    "noise",
    "noisy-regression",
    "status-regression",
    "stderr-diff",
    "large-body",
    "nested-json-diff",
    "ignored-dynamic-json",
    "large-json-match",
    "large-json-diff",
    "large-stderr-match",
    "serial-targets",
    "truncated-capture",
]


def shell_json(value):
    return "printf '%s\\n' '{}'".format(value)


def argv_json(*args):
    return json.dumps(list(args))


def python_print(value):
    return "python3 -c {}".format(
        shlex.quote(f"print({json.dumps(value)}, end='')")
    )


def repeated_json(value, count):
    return json.dumps({"items": [{"index": index, "value": value} for index in range(count)]})


SCENARIOS = {
    "match": {
        "expected": "match",
        "args": [
            "--primary",
            shell_json('{"value":42}'),
            "--candidate",
            shell_json('{"value":42}'),
            "--secondary",
            shell_json('{"value":42}'),
        ],
    },
    "match-argv": {
        "expected": "match",
        "args": [
            "--primary-argv",
            argv_json("printf", "%s\n", '{"value":42}'),
            "--candidate-argv",
            argv_json("printf", "%s\n", '{"value":42}'),
            "--secondary-argv",
            argv_json("printf", "%s\n", '{"value":42}'),
        ],
    },
    "candidate-diff": {
        "expected": "suspicious_difference",
        "args": [
            "--primary",
            shell_json('{"value":42}'),
            "--candidate",
            shell_json('{"value":43}'),
        ],
    },
    "noise": {
        "expected": "reference_noise",
        "args": [
            "--primary",
            shell_json('{"region":"a","value":1}'),
            "--candidate",
            shell_json('{"region":"a","value":1}'),
            "--secondary",
            shell_json('{"region":"b","value":1}'),
        ],
    },
    "noisy-regression": {
        "expected": "suspicious_with_noise",
        "args": [
            "--primary",
            shell_json('{"region":"a","total":42}'),
            "--candidate",
            shell_json('{"region":"a","total":99}'),
            "--secondary",
            shell_json('{"region":"b","total":42}'),
        ],
    },
    "status-regression": {
        "expected": "suspicious_difference",
        "args": [
            "--primary",
            shell_json('{"ok":true}'),
            "--candidate",
            "printf '%s\\n' '{\"ok\":true}'; exit 2",
        ],
    },
    "stderr-diff": {
        "expected": "suspicious_difference",
        "args": [
            "--primary",
            "printf '%s\\n' ok; printf '%s' primary-error >&2",
            "--candidate",
            "printf '%s\\n' ok; printf '%s' candidate-error >&2",
        ],
    },
    "large-body": {
        "expected": "match",
        "args": [
            "--primary",
            "python3 -c 'print(\"a\" * 65536, end=\"\")'",
            "--candidate",
            "python3 -c 'print(\"a\" * 65536, end=\"\")'",
        ],
    },
    "nested-json-diff": {
        "expected": "suspicious_difference",
        "args": [
            "--primary",
            shell_json('{"outer":{"items":[{"id":1,"value":"same"},{"id":2,"value":"old"}]}}'),
            "--candidate",
            shell_json('{"outer":{"items":[{"id":1,"value":"same"},{"id":2,"value":"new"}]}}'),
        ],
    },
    "ignored-dynamic-json": {
        "expected": "match",
        "args": [
            "--primary",
            shell_json('{"dynamic":"one","stable":true}'),
            "--candidate",
            shell_json('{"dynamic":"two","stable":true}'),
            "--ignore-json-path",
            "$.dynamic",
        ],
    },
    "large-json-match": {
        "expected": "match",
        "args": [
            "--primary",
            python_print(repeated_json("same", 512)),
            "--candidate",
            python_print(repeated_json("same", 512)),
        ],
    },
    "large-json-diff": {
        "expected": "suspicious_difference",
        "args": [
            "--primary",
            python_print(repeated_json("same", 512)),
            "--candidate",
            python_print(repeated_json("changed", 512)),
        ],
    },
    "large-stderr-match": {
        "expected": "match",
        "args": [
            "--primary",
            "python3 -c 'import sys; print(\"ok\"); sys.stderr.write(\"e\" * 65536)'",
            "--candidate",
            "python3 -c 'import sys; print(\"ok\"); sys.stderr.write(\"e\" * 65536)'",
        ],
    },
    "serial-targets": {
        "expected": "match",
        "args": [
            "--primary",
            shell_json('{"value":42}'),
            "--candidate",
            shell_json('{"value":42}'),
            "--serial-targets",
        ],
    },
    "truncated-capture": {
        "expected": "match",
        "args": [
            "--primary",
            "python3 -c 'print(\"abcdef\" * 4096, end=\"\")'",
            "--candidate",
            "python3 -c 'print(\"abcdef\" * 4096, end=\"\")'",
            "--max-body-capture-bytes",
            "32",
        ],
    },
}


def parse_args():
    parser = argparse.ArgumentParser(description="Benchmark moonlight CLI scenarios.")
    parser.add_argument(
        "--bin",
        default=os.getenv("MOONLIGHT_CLI_BIN", "target/release/moonlight"),
        help="Path to the moonlight binary.",
    )
    parser.add_argument(
        "--baseline-bin",
        default=os.getenv("MOONLIGHT_CLI_BASELINE_BIN"),
        help="Optional baseline moonlight binary to run side-by-side with --bin.",
    )
    parser.add_argument("--warmup", type=int, default=int(os.getenv("BENCHMARK_WARMUP", "20")))
    parser.add_argument(
        "--requests", type=int, default=int(os.getenv("BENCHMARK_REQUESTS", "200"))
    )
    parser.add_argument(
        "--concurrency", type=int, default=int(os.getenv("BENCHMARK_CONCURRENCY", "1"))
    )
    parser.add_argument(
        "--output-dir",
        default=os.getenv("BENCHMARK_OUTPUT_DIR", "data/moonlight/cli-benchmark-analysis"),
    )
    parser.add_argument(
        "--scenario",
        dest="scenarios",
        action="append",
        choices=sorted(SCENARIOS.keys()),
        help="Scenario to include; may be specified multiple times.",
    )
    parser.add_argument(
        "--target",
        dest="targets",
        action="append",
        choices=DEFAULT_TARGETS,
        help="Comparison target to include; may be specified multiple times. Defaults to all.",
    )
    parser.add_argument(
        "--comparison-runs",
        type=int,
        default=int(os.getenv("BENCHMARK_COMPARISON_RUNS", "20")),
        help="Measured suite invocations for each comparison target.",
    )
    parser.add_argument(
        "--comparison-cases",
        type=int,
        default=int(os.getenv("BENCHMARK_COMPARISON_CASES", "25")),
        help="Command-output cases per comparison suite invocation.",
    )
    return parser.parse_args()


def command_env():
    env = os.environ.copy()
    env.setdefault("LC_ALL", "C")
    return env


def run_cli_args_once(binary, storage_path, args):
    started = time.perf_counter()
    result = subprocess.run(
        [binary, "run", "--storage-path", str(storage_path), "--compact", *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=command_env(),
    )
    elapsed_ms = (time.perf_counter() - started) * 1000
    record = None
    parse_error = None
    if result.returncode == 0:
        try:
            record = json.loads(result.stdout)
        except json.JSONDecodeError as exc:
            parse_error = f"invalid stdout json: {exc}"
    return {
        "returncode": result.returncode,
        "latency_ms": elapsed_ms,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "record": record,
        "parse_error": parse_error,
    }


def run_cli_once(binary, storage_path, scenario):
    return run_cli_args_once(binary, storage_path, SCENARIOS[scenario]["args"])


def run_many(binary, storage_path, scenario, total, concurrency):
    results = []
    if total == 0:
        return results
    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = [
            executor.submit(run_cli_once, binary, storage_path, scenario)
            for _ in range(total)
        ]
        for future in concurrent.futures.as_completed(futures):
            results.append(future.result())
    return results


def run_command_once(command, cwd=None, env=None):
    started = time.perf_counter()
    result = subprocess.run(
        command,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env or command_env(),
    )
    elapsed_ms = (time.perf_counter() - started) * 1000
    return {
        "returncode": result.returncode,
        "latency_ms": elapsed_ms,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }


def run_command_many(command_factory, total, concurrency):
    results = []
    if total == 0:
        return results
    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = [
            executor.submit(lambda: run_command_once(*command_factory()))
            for _ in range(total)
        ]
        for future in concurrent.futures.as_completed(futures):
            results.append(future.result())
    return results


def percentile(values, percentile_value):
    if not values:
        return None
    ordered = sorted(values)
    index = (len(ordered) - 1) * (percentile_value / 100)
    lower = int(index)
    upper = min(lower + 1, len(ordered) - 1)
    if lower == upper:
        return ordered[lower]
    weight = index - lower
    return ordered[lower] * (1 - weight) + ordered[upper] * weight


def latency_summary(results):
    values = [result["latency_ms"] for result in results if result["returncode"] == 0]
    return {
        "min": min(values) if values else None,
        "mean": statistics.fmean(values) if values else None,
        "p50": percentile(values, 50),
        "p90": percentile(values, 90),
        "p95": percentile(values, 95),
        "p99": percentile(values, 99),
        "max": max(values) if values else None,
    }


def read_jsonl(path):
    records = []
    if not path.exists():
        return records
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError as exc:
                raise RuntimeError(f"{path}:{line_number}: invalid jsonl record: {exc}") from exc
    return records


def fetch_stats(binary, storage_path):
    result = subprocess.run(
        [binary, "stats", "--storage-path", str(storage_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=command_env(),
    )
    if result.returncode != 0:
        raise RuntimeError(f"stats failed for {storage_path}: {result.stderr}")
    return json.loads(result.stdout)


def classify_counts(records):
    counts = {}
    for record in records:
        classification = record["comparison"]["classification"]
        counts[classification] = counts.get(classification, 0) + 1
    return counts


def summarize_scenario(binary, output_dir, scenario, warmup, requests, concurrency):
    scenario_dir = output_dir / "scenarios" / scenario
    warmup_dir = output_dir / "warmup" / scenario
    scenario_dir.mkdir(parents=True, exist_ok=True)
    warmup_dir.mkdir(parents=True, exist_ok=True)
    warmup_storage = warmup_dir / "runs.jsonl"
    storage_path = scenario_dir / "runs.jsonl"
    for path in (warmup_storage, storage_path):
        if path.exists():
            path.unlink()

    print(f"warming {scenario} ({warmup} invocations)")
    run_many(binary, warmup_storage, scenario, warmup, concurrency)

    print(f"measuring {scenario} ({requests} invocations)")
    results = run_many(binary, storage_path, scenario, requests, concurrency)
    success_count = sum(1 for result in results if result["returncode"] == 0)
    error_count = len(results) - success_count
    parse_errors = [result["parse_error"] for result in results if result["parse_error"]]
    records = read_jsonl(storage_path)
    stats = fetch_stats(binary, storage_path)
    classifications = classify_counts(records)
    expected = SCENARIOS[scenario]["expected"]
    validation_errors = []

    if error_count:
        validation_errors.append(f"{error_count} invocation(s) exited non-zero")
    if parse_errors:
        validation_errors.extend(parse_errors[:5])
    if len(records) != success_count:
        validation_errors.append(
            f"expected {success_count} JSONL records, found {len(records)}"
        )
    expected_classifications = {} if success_count == 0 else {expected: success_count}
    if classifications != expected_classifications:
        validation_errors.append(
            f"expected classifications {expected_classifications}, found {classifications}"
        )
    if stats.get("total_runs") != len(records):
        validation_errors.append(
            f"stats total_runs {stats.get('total_runs')} did not match {len(records)}"
        )

    return {
        "total_invocations": len(results),
        "success_count": success_count,
        "error_count": error_count,
        "records_written": len(records),
        "storage_bytes": storage_path.stat().st_size if storage_path.exists() else 0,
        "classifications": classifications,
        "latency_ms": latency_summary(results),
        "validation_errors": validation_errors,
    }


def tool_version(command):
    path = shutil.which(command[0])
    if not path:
        return None
    result = subprocess.run(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=command_env(),
    )
    if result.returncode != 0:
        return f"{path}: version unavailable"
    first_line = result.stdout.strip().splitlines()
    return first_line[0] if first_line else path


def comparison_case_command():
    return "printf '%s\\n' '{\"value\":42}'"


def comparison_case_argv():
    return ["printf", "%s\n", comparison_expected_stdout()]


def comparison_expected_stdout():
    return '{"value":42}'


def prepare_cram_fixture(output_dir, cases):
    fixture_dir = output_dir / "comparison-fixtures" / "cram"
    fixture_dir.mkdir(parents=True, exist_ok=True)
    fixture = fixture_dir / "generated.t"
    lines = []
    for _ in range(cases):
        lines.append(f"  $ {comparison_case_command()}")
        lines.append(f"  {comparison_expected_stdout()}")
    fixture.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return fixture


def prepare_bats_fixture(output_dir, cases):
    fixture_dir = output_dir / "comparison-fixtures" / "bats"
    fixture_dir.mkdir(parents=True, exist_ok=True)
    fixture = fixture_dir / "generated.bats"
    command = comparison_case_command()
    expected_stdout = comparison_expected_stdout()
    lines = []
    for index in range(cases):
        lines.extend(
            [
                f'@test "case {index:04}" {{',
                f"  run sh -lc {shlex.quote(command)}",
                '  [ "$status" -eq 0 ]',
                f'  [ "$output" = {shlex.quote(expected_stdout)} ]',
                "}",
                "",
            ]
        )
    fixture.write_text("\n".join(lines), encoding="utf-8")
    return fixture


def prepare_shellspec_fixture(output_dir, cases):
    fixture_dir = output_dir / "comparison-fixtures" / "shellspec"
    spec_dir = fixture_dir / "spec"
    spec_dir.mkdir(parents=True, exist_ok=True)
    fixture = spec_dir / "generated_spec.sh"
    command = comparison_case_command()
    expected_stdout = comparison_expected_stdout()
    lines = ["Describe 'generated command-output cases'"]
    for index in range(cases):
        lines.extend(
            [
                f"  It 'case {index:04}'",
                f"    When run command sh -lc {shlex.quote(command)}",
                "    The status should equal 0",
                f"    The output should equal {shlex.quote(expected_stdout)}",
                "  End",
            ]
        )
    lines.append("End")
    fixture.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return fixture_dir


def toml_string(value):
    return json.dumps(value)


def prepare_trycmd_harness(output_dir, cases):
    harness_dir = output_dir / "comparison-fixtures" / "trycmd"
    cmd_dir = harness_dir / "tests" / "cmd"
    tests_dir = harness_dir / "tests"
    cmd_dir.mkdir(parents=True, exist_ok=True)
    tests_dir.mkdir(parents=True, exist_ok=True)
    for stale_case in cmd_dir.glob("case-*.toml"):
        stale_case.unlink()
    (harness_dir / "Cargo.toml").write_text(
        "\n".join(
            [
                "[package]",
                'name = "moonlight-cli-trycmd-benchmark"',
                'version = "0.0.0"',
                'edition = "2021"',
                "",
                "[workspace]",
                "",
                "[dev-dependencies]",
                'trycmd = "1.2.0"',
                "",
            ]
        ),
        encoding="utf-8",
    )
    (tests_dir / "cli_tests.rs").write_text(
        "\n".join(
            [
                "#[test]",
                "fn generated_cases() {",
                '    trycmd::TestCases::new().case("tests/cmd/*.toml");',
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    shell = shutil.which("sh") or "/bin/sh"
    command = comparison_case_command()
    expected_stdout = '{"value":42}\n'
    for index in range(cases):
        (cmd_dir / f"case-{index:04}.toml").write_text(
            "\n".join(
                [
                    f"bin.path = {toml_string(shell)}",
                    f"args = [{toml_string('-lc')}, {toml_string(command)}]",
                    f"stdout = {toml_string(expected_stdout)}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
    return harness_dir


def prepare_insta_harness(output_dir, cases):
    harness_dir = output_dir / "comparison-fixtures" / "insta"
    tests_dir = harness_dir / "tests"
    tests_dir.mkdir(parents=True, exist_ok=True)
    (harness_dir / "Cargo.toml").write_text(
        "\n".join(
            [
                "[package]",
                'name = "moonlight-cli-insta-benchmark"',
                'version = "0.0.0"',
                'edition = "2021"',
                "",
                "[workspace]",
                "",
                "[dev-dependencies]",
                'insta = "1.48.0"',
                "",
            ]
        ),
        encoding="utf-8",
    )
    command = comparison_case_command()
    expected_stdout = comparison_expected_stdout()
    (tests_dir / "insta_cmd_tests.rs").write_text(
        "\n".join(
            [
                "use std::process::Command;",
                "",
                "#[test]",
                "fn generated_cases() {",
                f"    let command = {json.dumps(command)};",
                f"    let expected_stdout = {json.dumps(expected_stdout)};",
                f"    for index in 0..{cases} {{",
                '        let output = Command::new("sh")',
                '            .arg("-lc")',
                "            .arg(command)",
                "            .output()",
                '            .expect("run generated command");',
                '        assert!(output.status.success(), "case {index:04} exited with {:?}", output.status.code());',
                "        let stdout = String::from_utf8(output.stdout).expect(\"stdout should be UTF-8\");",
                "        assert_eq!(stdout.trim_end(), expected_stdout);",
                "        insta::with_settings!({ snapshot_suffix => format!(\"case_{index:04}\") }, {",
                "            insta::allow_duplicates! {",
                f"                insta::assert_snapshot!(stdout.trim_end(), @r###\"{expected_stdout}\"###);",
                "            }",
                "        });",
                "    }",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return harness_dir


def build_cargo_test_harness(harness_dir, binary_prefix):
    build = run_command_once(["cargo", "test", "--quiet", "--no-run"], cwd=harness_dir)
    if build["returncode"] != 0:
        return None, build["stderr"] or build["stdout"]
    candidates = [
        Path(path)
        for path in glob.glob(str(harness_dir / "target" / "debug" / "deps" / f"{binary_prefix}-*"))
        if not path.endswith(".d") and os.access(path, os.X_OK)
    ]
    if not candidates:
        return None, f"{binary_prefix} test binary was not produced"
    return max(candidates, key=lambda path: path.stat().st_mtime).resolve(), None


def target_latency_results(results, target_invocations_per_case, cases):
    denominator = target_invocations_per_case * cases
    if denominator <= 0:
        return []
    return [
        {**result, "latency_ms": result["latency_ms"] / denominator}
        for result in results
        if result["returncode"] == 0
    ]


def add_target_invocation_metrics(comparison, target_invocations_per_case, results=None):
    if target_invocations_per_case < 1:
        raise ValueError("target_invocations_per_case must be at least 1")
    comparison["target_invocations_per_case"] = target_invocations_per_case
    comparison["total_target_invocations"] = (
        comparison["success_count"]
        * comparison["cases_per_invocation"]
        * target_invocations_per_case
    )
    comparison["target_invocation_latency_ms"] = latency_summary(
        target_latency_results(results or [], target_invocations_per_case, comparison["cases_per_invocation"])
    )
    return comparison


def summarize_comparison_target(
    name,
    cases,
    warmup,
    runs,
    concurrency,
    command_factory,
    target_invocations_per_case=1,
):
    print(f"warming {name} comparison ({warmup} suite invocations)")
    warmup_results = run_command_many(command_factory, warmup, concurrency)
    warmup_errors = [result for result in warmup_results if result["returncode"] != 0]
    if warmup_errors:
        error = warmup_errors[0]
        return add_target_invocation_metrics({
            "status": "failed",
            "reason": (error["stderr"] or error["stdout"]).strip()[:1000],
            "cases_per_invocation": cases,
            "total_invocations": 0,
            "success_count": 0,
            "error_count": len(warmup_errors),
            "total_cases": 0,
            "latency_ms": latency_summary([]),
            "case_latency_ms": latency_summary([]),
            "validation_errors": [f"{name} warmup failed"],
        }, target_invocations_per_case)

    print(f"measuring {name} comparison ({runs} suite invocations)")
    results = run_command_many(command_factory, runs, concurrency)
    success_count = sum(1 for result in results if result["returncode"] == 0)
    error_count = len(results) - success_count
    validation_errors = []
    if error_count:
        validation_errors.append(f"{error_count} {name} comparison invocation(s) exited non-zero")

    case_results = [
        {**result, "latency_ms": result["latency_ms"] / cases}
        for result in results
        if result["returncode"] == 0 and cases > 0
    ]
    return add_target_invocation_metrics({
        "status": "ok" if not validation_errors else "failed",
        "reason": None,
        "cases_per_invocation": cases,
        "total_invocations": len(results),
        "success_count": success_count,
        "error_count": error_count,
        "total_cases": success_count * cases,
        "latency_ms": latency_summary(results),
        "case_latency_ms": latency_summary(case_results),
        "validation_errors": validation_errors,
    }, target_invocations_per_case, results)


def summarize_moonlight_comparison(binary, output_dir, cases, warmup, runs, concurrency, command_mode):
    suite_name = "moonlight" if command_mode == "shell" else "moonlight-argv"
    suite_dir = output_dir / "comparison-fixtures" / suite_name
    suite_dir.mkdir(parents=True, exist_ok=True)
    for stale_storage in suite_dir.glob("*.jsonl"):
        stale_storage.unlink()
    input_path = suite_dir / "cases.jsonl"
    if command_mode == "argv":
        case = {
            "primary_argv": comparison_case_argv(),
            "candidate_argv": comparison_case_argv(),
        }
    else:
        case = {
            "primary": comparison_case_command(),
            "candidate": comparison_case_command(),
        }
    input_path.write_text(
        "\n".join(json.dumps(case, sort_keys=True) for _ in range(cases)) + "\n",
        encoding="utf-8",
    )

    def run_suite():
        storage = suite_dir / f"{uuid.uuid4()}.jsonl"
        result = run_command_once(
            [
                binary,
                "batch",
                "--input",
                str(input_path),
                "--storage-path",
                str(storage),
                "--quiet",
            ]
        )
        if result["returncode"] != 0:
            return result
        records = read_jsonl(storage)
        if len(records) != cases:
            result["returncode"] = 1
            result["stderr"] = f"expected {cases} JSONL records, found {len(records)}"
            return result
        classifications = classify_counts(records)
        if classifications != {"match": cases}:
            result["returncode"] = 1
            result["stderr"] = f"expected {cases} match records, found {classifications}"
            return result
        return result

    def runner():
        return run_suite()

    def many(total):
        results = []
        if total == 0:
            return results
        with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
            futures = [executor.submit(runner) for _ in range(total)]
            for future in concurrent.futures.as_completed(futures):
                results.append(future.result())
        return results

    print(f"warming {suite_name} comparison ({warmup} suite invocations)")
    warmup_results = many(warmup)
    warmup_errors = [result for result in warmup_results if result["returncode"] != 0]
    if warmup_errors:
        error = warmup_errors[0]
        return add_target_invocation_metrics({
            "status": "failed",
            "reason": (error["stderr"] or error["stdout"]).strip()[:1000],
            "cases_per_invocation": cases,
            "total_invocations": 0,
            "success_count": 0,
            "error_count": len(warmup_errors),
            "total_cases": 0,
            "latency_ms": latency_summary([]),
            "case_latency_ms": latency_summary([]),
            "validation_errors": [f"{suite_name} comparison warmup failed"],
        }, 2)

    print(f"measuring {suite_name} comparison ({runs} suite invocations)")
    results = many(runs)
    success_count = sum(1 for result in results if result["returncode"] == 0)
    error_count = len(results) - success_count
    validation_errors = []
    if error_count:
        validation_errors.append(f"{error_count} {suite_name} comparison invocation(s) exited non-zero")
    case_results = [
        {**result, "latency_ms": result["latency_ms"] / cases}
        for result in results
        if result["returncode"] == 0 and cases > 0
    ]
    return add_target_invocation_metrics({
        "status": "ok" if not validation_errors else "failed",
        "reason": None,
        "cases_per_invocation": cases,
        "total_invocations": len(results),
        "success_count": success_count,
        "error_count": error_count,
        "total_cases": success_count * cases,
        "latency_ms": latency_summary(results),
        "case_latency_ms": latency_summary(case_results),
        "validation_errors": validation_errors,
    }, 2, results)


def skipped_comparison(reason, cases, target_invocations_per_case=1):
    return add_target_invocation_metrics({
        "status": "skipped",
        "reason": reason,
        "cases_per_invocation": cases,
        "total_invocations": 0,
        "success_count": 0,
        "error_count": 0,
        "total_cases": 0,
        "latency_ms": latency_summary([]),
        "case_latency_ms": latency_summary([]),
        "validation_errors": [],
    }, target_invocations_per_case)


def summarize_moonlight_target(binary, output_dir, cases, warmup, runs, concurrency):
    comparison = summarize_moonlight_comparison(
        str(binary), output_dir, cases, warmup, runs, concurrency, "shell"
    )
    comparison["version"] = f"{binary} batch"
    return comparison


def summarize_moonlight_argv_target(binary, output_dir, cases, warmup, runs, concurrency):
    comparison = summarize_moonlight_comparison(
        str(binary), output_dir, cases, warmup, runs, concurrency, "argv"
    )
    comparison["version"] = f"{binary} batch argv"
    return comparison


def summarize_cram_target(_binary, output_dir, cases, warmup, runs, concurrency):
    version = tool_version(["cram", "--version"])
    if not shutil.which("cram"):
        comparison = skipped_comparison("cram executable not found on PATH", cases)
        comparison["version"] = None
        return comparison

    fixture = prepare_cram_fixture(output_dir, cases)

    def cram_command():
        return (["cram", str(fixture)], None, command_env())

    comparison = summarize_comparison_target(
        "cram", cases, warmup, runs, concurrency, cram_command
    )
    comparison["version"] = version
    return comparison


def summarize_trycmd_target(_binary, output_dir, cases, warmup, runs, concurrency):
    cargo_version = tool_version(["cargo", "-V"])
    if not shutil.which("cargo"):
        comparison = skipped_comparison("cargo executable not found on PATH", cases)
        comparison["version"] = None
        return comparison

    harness_dir = prepare_trycmd_harness(output_dir, cases)
    test_binary, build_error = build_cargo_test_harness(harness_dir, "cli_tests")
    if build_error:
        comparison = skipped_comparison(f"trycmd harness build failed: {build_error}", cases)
        comparison["version"] = cargo_version
        return comparison

    env = command_env()
    env.setdefault("CARGO_TERM_COLOR", "never")

    def trycmd_command():
        return ([str(test_binary)], harness_dir, env)

    comparison = summarize_comparison_target(
        "trycmd", cases, warmup, runs, concurrency, trycmd_command
    )
    comparison["version"] = f"trycmd 1.2.0 via {cargo_version}"
    return comparison


def summarize_insta_target(_binary, output_dir, cases, warmup, runs, concurrency):
    cargo_version = tool_version(["cargo", "-V"])
    if not shutil.which("cargo"):
        comparison = skipped_comparison("cargo executable not found on PATH", cases)
        comparison["version"] = None
        return comparison

    harness_dir = prepare_insta_harness(output_dir, cases)
    test_binary, build_error = build_cargo_test_harness(harness_dir, "insta_cmd_tests")
    if build_error:
        comparison = skipped_comparison(f"insta harness build failed: {build_error}", cases)
        comparison["version"] = cargo_version
        return comparison

    env = command_env()
    env.setdefault("CARGO_TERM_COLOR", "never")
    env.setdefault("INSTA_UPDATE", "no")
    env.setdefault("NO_COLOR", "1")

    def insta_command():
        return ([str(test_binary)], harness_dir, env)

    comparison = summarize_comparison_target(
        "insta", cases, warmup, runs, concurrency, insta_command
    )
    comparison["version"] = f"insta 1.48.0 via {cargo_version}"
    return comparison


def summarize_bats_target(_binary, output_dir, cases, warmup, runs, concurrency):
    version = tool_version(["bats", "--version"])
    if not shutil.which("bats"):
        comparison = skipped_comparison("bats executable not found on PATH", cases)
        comparison["version"] = None
        return comparison

    fixture = prepare_bats_fixture(output_dir, cases)

    def bats_command():
        return (["bats", str(fixture)], None, command_env())

    comparison = summarize_comparison_target(
        "bats", cases, warmup, runs, concurrency, bats_command
    )
    comparison["version"] = version
    return comparison


def summarize_shellspec_target(_binary, output_dir, cases, warmup, runs, concurrency):
    version = tool_version(["shellspec", "--version"])
    if not shutil.which("shellspec"):
        comparison = skipped_comparison("shellspec executable not found on PATH", cases)
        comparison["version"] = None
        return comparison

    fixture_dir = prepare_shellspec_fixture(output_dir, cases)

    def shellspec_command():
        return (["shellspec", "--format", "dot"], fixture_dir, command_env())

    comparison = summarize_comparison_target(
        "shellspec", cases, warmup, runs, concurrency, shellspec_command
    )
    comparison["version"] = version
    return comparison


COMPARISON_TARGETS = {
    "moonlight": summarize_moonlight_target,
    "moonlight-argv": summarize_moonlight_argv_target,
    "trycmd": summarize_trycmd_target,
    "insta": summarize_insta_target,
    "cram": summarize_cram_target,
    "bats": summarize_bats_target,
    "shellspec": summarize_shellspec_target,
}


def summarize_comparisons(binary, output_dir, targets, cases, warmup, runs, concurrency):
    comparisons = {}
    for target in targets:
        comparisons[target] = COMPARISON_TARGETS[target](
            binary, output_dir, cases, warmup, runs, concurrency
        )

    return comparisons


def summarize_baseline_comparisons(
    baseline_binary, output_dir, targets, cases, warmup, runs, concurrency
):
    comparisons = {}
    if "moonlight" in targets:
        comparisons["baseline-moonlight"] = summarize_moonlight_target(
            baseline_binary, output_dir, cases, warmup, runs, concurrency
        )
    if "moonlight-argv" in targets:
        comparisons["baseline-moonlight-argv"] = summarize_moonlight_argv_target(
            baseline_binary, output_dir, cases, warmup, runs, concurrency
        )
    return comparisons


def run_text(command):
    try:
        return subprocess.check_output(command, text=True, stderr=subprocess.STDOUT).strip()
    except Exception as exc:
        return f"unavailable: {exc}"


def environment(binary, baseline_binary=None):
    output = {
        "git_sha": run_text(["git", "rev-parse", "HEAD"]),
        "rustc": run_text(["rustc", "-V"]),
        "cargo": run_text(["cargo", "-V"]),
        "binary": str(binary),
    }
    if baseline_binary is not None:
        output["baseline_binary"] = str(baseline_binary)
    return output


def format_ms(value):
    return "-" if value is None else f"{value:.2f}"


def markdown_cell(value):
    return str(value).replace("\\", "\\\\").replace("|", "\\|")


def write_markdown(report, path):
    lines = [
        "# moonlight CLI Benchmark",
        "",
        f"Generated: `{report['generated_at']}`",
        "",
        "## Scenarios",
        "",
        "| Scenario | Invocations | Success | Errors | Records | p50 ms | p95 ms | p99 ms | Mean ms | Max ms |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for name, scenario in report["scenarios"].items():
        latency = scenario["latency_ms"]
        lines.append(
            "| {name} | {total} | {success} | {errors} | {records} | {p50} | {p95} | {p99} | {mean} | {max} |".format(
                name=name,
                total=scenario["total_invocations"],
                success=scenario["success_count"],
                errors=scenario["error_count"],
                records=scenario["records_written"],
                p50=format_ms(latency["p50"]),
                p95=format_ms(latency["p95"]),
                p99=format_ms(latency["p99"]),
                mean=format_ms(latency["mean"]),
                max=format_ms(latency["max"]),
            )
        )

    lines.extend(["", "## Classifications", "", "| Scenario | Counts |", "| --- | --- |"])
    for name, scenario in report["scenarios"].items():
        lines.append(f"| {name} | `{json.dumps(scenario['classifications'], sort_keys=True)}` |")

    lines.extend(
        [
            "",
            "## Tool Comparisons",
            "",
            "Each comparison case is a deterministic shell command-output check; moonlight runs a primary/candidate comparison, while the other targets run snapshot-style assertions when available.",
            "",
            "| Target | Status | Suite Runs | Cases/Run | Target invocations/case | Total Cases | Total Target Invocations | Suite p50 ms | Suite p95 ms | Per-case p50 ms | Per-case p95 ms | Per-target p50 ms | Per-target p95 ms | Version/Reason |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
        ]
    )
    for name, comparison in report["comparisons"].items():
        suite_latency = comparison["latency_ms"]
        case_latency = comparison["case_latency_ms"]
        target_latency = comparison["target_invocation_latency_ms"]
        detail = comparison.get("reason") or comparison.get("version") or ""
        detail = detail.replace("\n", " ")[:180]
        detail = markdown_cell(detail)
        lines.append(
            "| {name} | {status} | {runs} | {cases} | {target_invocations} | {total_cases} | {total_target_invocations} | {suite_p50} | {suite_p95} | {case_p50} | {case_p95} | {target_p50} | {target_p95} | {detail} |".format(
                name=name,
                status=comparison["status"],
                runs=comparison["total_invocations"],
                cases=comparison["cases_per_invocation"],
                target_invocations=comparison["target_invocations_per_case"],
                total_cases=comparison["total_cases"],
                total_target_invocations=comparison["total_target_invocations"],
                suite_p50=format_ms(suite_latency["p50"]),
                suite_p95=format_ms(suite_latency["p95"]),
                case_p50=format_ms(case_latency["p50"]),
                case_p95=format_ms(case_latency["p95"]),
                target_p50=format_ms(target_latency["p50"]),
                target_p95=format_ms(target_latency["p95"]),
                detail=detail,
            )
        )

    validation_errors = [
        error
        for scenario in report["scenarios"].values()
        for error in scenario["validation_errors"]
    ] + [
        error
        for comparison in report["comparisons"].values()
        for error in comparison["validation_errors"]
    ]
    lines.extend(["", "## Validation", ""])
    if validation_errors:
        lines.extend(f"- {error}" for error in validation_errors)
    else:
        lines.append("All validations passed.")

    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main():
    args = parse_args()
    if args.warmup < 0:
        raise SystemExit("--warmup must be non-negative")
    if args.requests < 0:
        raise SystemExit("--requests must be non-negative")
    if args.concurrency < 1:
        raise SystemExit("--concurrency must be at least 1")
    if args.comparison_runs < 0:
        raise SystemExit("--comparison-runs must be non-negative")
    if args.comparison_cases < 1:
        raise SystemExit("--comparison-cases must be at least 1")

    binary = Path(args.bin)
    if not binary.exists():
        raise SystemExit(f"moonlight binary not found: {binary}")
    baseline_binary = Path(args.baseline_bin) if args.baseline_bin else None
    if baseline_binary is not None and not baseline_binary.exists():
        raise SystemExit(f"baseline moonlight binary not found: {baseline_binary}")

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    scenarios = args.scenarios or DEFAULT_SCENARIOS
    targets = args.targets or DEFAULT_TARGETS

    report = {
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "environment": environment(binary, baseline_binary),
        "config": {
            "warmup": args.warmup,
            "requests": args.requests,
            "concurrency": args.concurrency,
            "scenarios": scenarios,
            "targets": targets,
            "baseline_bin": str(baseline_binary) if baseline_binary else None,
            "comparison_runs": args.comparison_runs,
            "comparison_cases": args.comparison_cases,
        },
        "scenarios": {},
        "comparisons": {},
    }

    candidate_output_dir = output_dir / "candidate" if baseline_binary is not None else output_dir
    for scenario in scenarios:
        report["scenarios"][scenario] = summarize_scenario(
            str(binary),
            candidate_output_dir,
            scenario,
            args.warmup,
            args.requests,
            args.concurrency,
        )
        if baseline_binary is not None:
            report["scenarios"][f"baseline-{scenario}"] = summarize_scenario(
                str(baseline_binary),
                output_dir / "baseline",
                scenario,
                args.warmup,
                args.requests,
                args.concurrency,
            )

    report["comparisons"] = summarize_comparisons(
        binary,
        candidate_output_dir,
        targets,
        args.comparison_cases,
        min(args.warmup, args.comparison_runs),
        args.comparison_runs,
        args.concurrency,
    )
    if baseline_binary is not None:
        report["comparisons"].update(
            summarize_baseline_comparisons(
                baseline_binary,
                output_dir / "baseline",
                targets,
                args.comparison_cases,
                min(args.warmup, args.comparison_runs),
                args.comparison_runs,
                args.concurrency,
            )
        )

    json_path = output_dir / "latest.json"
    markdown_path = output_dir / "latest.md"
    json_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    write_markdown(report, markdown_path)
    print(f"wrote {json_path}")
    print(f"wrote {markdown_path}")

    validation_errors = [
        error
        for scenario in report["scenarios"].values()
        for error in scenario["validation_errors"]
    ] + [
        error
        for comparison in report["comparisons"].values()
        for error in comparison["validation_errors"]
    ]
    if validation_errors:
        for error in validation_errors:
            print(f"validation error: {error}")
        raise SystemExit(1)


if __name__ == "__main__":
    main()
