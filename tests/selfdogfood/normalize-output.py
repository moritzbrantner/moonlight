#!/usr/bin/env python3
"""Run and normalize Moonlight self-dogfood cases."""
from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

VOLATILE_KEY_RE = re.compile(r"(^|_)(id|uuid|timestamp|started_at|finished_at|duration|elapsed|latency)(_ms|_ns|_s)?$|^(version)$", re.I)
UUID_RE = re.compile(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b", re.I)
TMP_RE = re.compile(r"/(?:tmp|var/folders)/[^\s\"']+")


def load_jsonish(text: str) -> Any:
    stripped = text.strip()
    if not stripped:
        return ""
    try:
        return normalize_value(json.loads(stripped))
    except json.JSONDecodeError:
        records = []
        for line in stripped.splitlines():
            try:
                records.append(normalize_value(json.loads(line)))
            except json.JSONDecodeError:
                return normalize_string(text)
        return records


def normalize_string(value: str) -> str:
    value = UUID_RE.sub("<uuid>", value)
    value = TMP_RE.sub("<tmp>", value)
    cwd = str(Path.cwd())
    value = value.replace(cwd, "<repo>")
    value = re.sub(r"(?:\./)?target/release/moonlight(?:\.exe)?", "<moonlight-bin>", value)
    return value


def normalize_value(value: Any) -> Any:
    if isinstance(value, dict):
        normalized = {}
        for key, child in value.items():
            if VOLATILE_KEY_RE.search(str(key)):
                normalized[key] = "<volatile>"
            else:
                normalized[key] = normalize_value(child)
        return normalized
    if isinstance(value, list):
        return [normalize_value(item) for item in value]
    if isinstance(value, str):
        return normalize_string(value)
    return value


def normalized_process_result(result: subprocess.CompletedProcess[str]) -> dict[str, Any]:
    return {
        "exit_code": result.returncode,
        "stdout": load_jsonish(result.stdout),
        "stderr": load_jsonish(result.stderr),
    }


def run_one(label: str, command: list[str], case_dir: Path) -> dict[str, Any]:
    result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    (case_dir / f"{label}.stdout").write_text(result.stdout)
    (case_dir / f"{label}.stderr").write_text(result.stderr)
    normalized = normalized_process_result(result)
    (case_dir / f"{label}.normalized.json").write_text(json.dumps(normalized, indent=2, sort_keys=True) + "\n")
    return normalized


def read_cases(path: Path) -> list[dict[str, Any]]:
    cases = []
    for line_number, line in enumerate(path.read_text().splitlines(), 1):
        if line.strip():
            case = json.loads(line)
            case.setdefault("id", f"line-{line_number}")
            cases.append(case)
    return cases


def materialize_args(case: dict[str, Any], case_dir: Path) -> list[str]:
    args = list(case["args"])
    if "batch_cases" in case:
        batch_input = case_dir / "batch-cases.jsonl"
        batch_input.write_text("".join(json.dumps(item, separators=(",", ":")) + "\n" for item in case["batch_cases"]))
        args = [str(batch_input) if arg == "{batch_input}" else arg for arg in args]
    return args


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--work-dir", required=True, type=Path)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--reference", required=True, nargs="+")
    ns = parser.parse_args()

    ns.work_dir.mkdir(parents=True, exist_ok=True)
    print("Moonlight selfdogfood: published vs source")
    print(f"candidate: {ns.candidate}")
    print(f"reference: {' '.join(ns.reference)}")
    print()

    passed = 0
    failed = 0
    for case in read_cases(ns.cases):
        case_id = case["id"]
        case_dir = ns.work_dir / case_id
        case_dir.mkdir(parents=True, exist_ok=True)
        args = materialize_args(case, case_dir)
        reference = run_one("published", ns.reference + args, case_dir)
        candidate = run_one("source", [ns.candidate] + args, case_dir)
        if reference == candidate:
            passed += 1
            print(f"PASS {case_id}")
        else:
            failed += 1
            diff = "".join(difflib.unified_diff(
                json.dumps(reference, indent=2, sort_keys=True).splitlines(True),
                json.dumps(candidate, indent=2, sort_keys=True).splitlines(True),
                fromfile="published.normalized.json",
                tofile="source.normalized.json",
            ))
            diff_path = case_dir / f"{case_id}.diff"
            diff_path.write_text(diff)
            print(f"FAIL {case_id}")
            print("  normalized output differs")
            print(f"  see: {diff_path}")
    print()
    print(f"{passed} passed, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
