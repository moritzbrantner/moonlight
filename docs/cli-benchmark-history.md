# CLI benchmark history

Moonlight publishes fresh CLI performance evidence from the default branch without writing generated benchmark results back to `main`.

## Measurement model

The `Publish CLI benchmark history` workflow runs after every push to `main` and can also be started manually. It builds `moonlight-cli` in release mode with the pinned Rust 1.96.0 toolchain and runs the full `scripts/moonlight-cli-benchmark.py` suite.

The benchmark keeps two Moonlight execution paths visible:

- `moonlight-argv` is the preferred path for trusted deterministic commands because it bypasses shell startup and parsing;
- `moonlight` remains the shell-command compatibility path.

The report retains suite, per-case, and per-target p50/p95 measurements for both paths. The history also records the direct-argv latency reduction relative to the shell path; this is descriptive benchmark evidence, not a universal performance guarantee.

## Persistence

Generated evidence is written to the dedicated `cli-benchmark-history` branch:

- `latest.json` is the latest exact-head full CLI benchmark report;
- `history.json` uses schema `moonlight/cli-benchmark-history/v1` and retains up to 1,000 commit-addressed compact snapshots.

Each commit appears at most once. Re-running a commit replaces its prior history entry rather than duplicating it. The existing committed June 2026 benchmark is retained as the initial historical anchor, while new exact-head measurements remain off `main`.

The workflow records the benchmark Rust/Cargo versions and source SHA with every snapshot. The benchmark toolchain is pinned so the trend does not silently move merely because the hosted runner's default Rust version changes.

## Pages

The GitHub Pages overview loads `latest.json` and `history.json` directly from the persistence branch. If that evidence is temporarily unavailable, the existing committed benchmark remains a visible fallback.

The overview:

- uses direct argv for the headline CLI p95 when it is available;
- explicitly shows its p95 reduction relative to shell-command execution;
- plots shell and argv p95 latency through the latest 60 measured commits;
- links the latest plotted point to its exact Git commit.

The machine-facing `agent-tool.json` catalog exposes both the latest published report and the commit history as read-only JSON resources. Pages and the history branch display captured evidence only; local/native Moonlight execution remains authoritative for producing new comparisons.
