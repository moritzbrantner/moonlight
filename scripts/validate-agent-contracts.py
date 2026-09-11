#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROVENANCE = ROOT / "contracts" / "agent-contracts" / "PROVENANCE.json"
EXPECTED_SOURCE = "https://github.com/moritzbrantner/agent-contracts"
EXPECTED_IDS = {
    "contracts/agent-contracts/evidence-v1.schema.json": "urn:agent-contracts:evidence:v1",
    "contracts/agent-contracts/evaluation-result-v1.schema.json": "urn:agent-contracts:evaluation-result:v1",
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    provenance = json.loads(PROVENANCE.read_text(encoding="utf-8"))
    if provenance.get("schemaVersion") != 1:
        raise SystemExit("agent-contracts provenance must use schemaVersion 1")
    if provenance.get("source") != EXPECTED_SOURCE:
        raise SystemExit("agent-contracts provenance source is not canonical")
    revision = provenance.get("revision")
    if not isinstance(revision, str) or len(revision) != 40:
        raise SystemExit("agent-contracts provenance revision must be an immutable Git SHA")

    entries = provenance.get("files")
    if not isinstance(entries, list) or not entries:
        raise SystemExit("agent-contracts provenance must declare pinned files")

    seen: set[str] = set()
    for entry in entries:
        local_path = entry.get("localPath")
        expected_sha = entry.get("sha256")
        source_path = entry.get("sourcePath")
        if not isinstance(local_path, str) or local_path in seen:
            raise SystemExit(f"invalid or duplicate localPath: {local_path!r}")
        seen.add(local_path)
        if not isinstance(source_path, str) or not source_path.startswith("schemas/"):
            raise SystemExit(f"invalid sourcePath for {local_path}")
        path = ROOT / local_path
        if not path.is_file():
            raise SystemExit(f"missing pinned contract: {local_path}")
        actual_sha = digest(path)
        if actual_sha != expected_sha:
            raise SystemExit(
                f"pinned contract drifted: {local_path}: expected {expected_sha}, got {actual_sha}"
            )
        schema = json.loads(path.read_text(encoding="utf-8"))
        expected_id = EXPECTED_IDS.get(local_path)
        if expected_id is None:
            raise SystemExit(f"unexpected pinned contract: {local_path}")
        if schema.get("$id") != expected_id:
            raise SystemExit(f"unexpected schema id for {local_path}: {schema.get('$id')!r}")

    if seen != set(EXPECTED_IDS):
        missing = sorted(set(EXPECTED_IDS) - seen)
        raise SystemExit(f"provenance does not cover the complete Moonlight contract seam: {missing}")

    evaluation = json.loads(
        (ROOT / "contracts/agent-contracts/evaluation-result-v1.schema.json").read_text(
            encoding="utf-8"
        )
    )
    evidence_ref = evaluation["properties"]["evidence"]["items"].get("$ref")
    if evidence_ref != EXPECTED_IDS["contracts/agent-contracts/evidence-v1.schema.json"]:
        raise SystemExit("evaluation-result no longer references the pinned evidence contract")

    print(f"Verified {len(seen)} agent contracts at {revision}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
