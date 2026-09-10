# CLI benchmark history

Moonlight publishes fresh CLI performance evidence from the default branch without writing generated benchmark results back to `main`.

## Measurement model

The `Publish CLI benchmark history` workflow runs after every push to `main` and can also be started manually. It builds `moonlight-cli` in release mode with the canonical toolchain from `rust-toolchain.toml`, currently Rust 1.98.1, and runs the full `scripts/moonlight-cli-benchmark.py` suite.

Direct argv is the canonical deterministic command path. The benchmark deliberately keeps the deprecated shell path beside it as a compatibility comparison so the cost of shell startup and parsing stays visible rather than being folded into the preferred-path number.

The report retains suite, per-case, and per-target p50/p95 measurements for both paths. The history also records the direct-argv latency reduction relative to the shell path; this is descriptive benchmark evidence, not a universal performance guarantee.

## Persistence

Generated evidence is written to the dedicated `cli-benchmark-history` branch:

- `latest.json` is the latest exact-head full CLI benchmark report;
- `history.json` uses schema `moonlight/cli-benchmark-history/v1` and retains up to 1,000 commit-addressed compact snapshots.

Each commit appears at most once. Re-running a commit replaces its prior history entry rather than duplicating it. The existing committed June 2026 benchmark is retained as the initial historical anchor, while new exact-head measurements remain off `main`.

The workflow records the benchmark Rust/Cargo versions and source SHA with every snapshot. Toolchain upgrades are therefore explicit history boundaries: a point measured with Rust 1.98.1 is not silently presented as if it used the older Rust 1.96.0 environment. Historical observations remain immutable evidence of the environment in which they were measured.

## Pages

The GitHub Pages overview loads `latest.json` and `history.json` directly from the persistence branch. If that evidence is temporarily unavailable, the existing committed benchmark remains a visible fallback.

The overview:

- uses direct argv for the headline CLI p95 when it is available;
- explicitly shows its p95 reduction relative to deprecated shell-command execution;
- plots shell and argv p95 latency through the latest 60 measured commits;
- links the latest plotted point to its exact Git commit.

The machine-facing `agent-tool.json` catalog exposes both the latest published report and the commit history as read-only JSON resources. Pages and the history branch display captured evidence only; local/native Moonlight execution remains authoritative for producing new comparisons.
