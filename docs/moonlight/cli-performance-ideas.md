# CLI Performance Ideas

This list ranks likely speedups for `moonlight-cli` and `moonlight-core`. Items marked done have an initial implementation; the remaining notes are follow-up work and metrics to watch.

## Done In Current Pass

- `run --compact` prints one-line JSON for machine consumers while preserving pretty JSON by default.
- `batch` accepts direct `*_argv` command forms for trusted deterministic fixtures.
- `run` accepts direct `--primary-argv`, `--candidate-argv`, and `--secondary-argv` command forms.
- Default batch cases reuse a shared `CompareConfig`.
- `batch` writes completed runs through a dedicated writer task.
- CLI command capture now reads stdout and stderr concurrently while computing SHA-256 and previews in one pass, while still retaining full bytes for current diff compatibility.
- CLI benchmark reports include normalized per-target invocation latency and a `moonlight-argv` comparison target.

## 1. Storage Fast Paths For CLI Read Commands

- Current behavior: `Storage::load` scans every `.jsonl` file in the storage directory, deserializes full `ComparisonRun` records, and then `list`, `stats`, and `show` derive their response from the in-memory vector.
- Expected impact: High for directories that contain HTTP and CLI benchmark outputs or many historical runs.
- Risk level: Medium. Admin views intentionally merge records across files today, so the fast path should be scoped to CLI commands that pass a concrete `--storage-path`.
- Metric to watch: `stats_1000_runs`, `list_1000_runs`, and `show_middle_run_1000_runs` Criterion timings, plus read-command latency in `scripts/moonlight-cli-benchmark.py`.
- Acceptance criterion: `stats`, `list`, and `show` read only the requested storage file in CLI fast-path mode, while existing merged-directory behavior remains available where the HTTP admin path needs it.

## 2. Streaming Command Capture

- Current behavior: `moonlight-cli` uses `tokio::process::Command::output()`, which buffers full stdout and stderr before Moonlight hashes, previews, stores, and compares them.
- Current status: initial streaming capture is implemented for CLI command execution, but full bytes are still retained for comparison compatibility.
- Expected impact: High for large stdout/stderr workloads and lower peak memory for failed or noisy command suites.
- Risk level: Medium. The comparison engine still needs full bytes for exact body and stderr comparisons unless a streaming equality/hash strategy is added carefully.
- Metric to watch: `large-body`, `large-json-match`, `large-json-diff`, `large-stderr-match`, and process memory during benchmark runs.
- Acceptance criterion: command capture maintains SHA-256, size, preview, status, stderr, and current classification behavior while avoiding full buffering for persisted observations where exact diffing does not need it.

## 3. Reuse `CompareConfig` In Batch Mode

- Current behavior: `execute_case` rebuilds ignored JSON path and ignored header hash sets for every batch case.
- Current status: default batch cases share a single default `CompareConfig`; customized cases still build per-case configs.
- Expected impact: Medium for large `batch` suites with simple command-output cases.
- Risk level: Low. Cases with custom ignored paths or headers still need per-case configs, but default cases can share one config.
- Metric to watch: `moonlight` tool-comparison per-case latency at 100+ cases per invocation.
- Acceptance criterion: default batch cases reuse a shared `CompareConfig`, custom cases still override correctly, and all CLI tests remain unchanged.

## 4. Add Direct Argv Execution Mode

- Previous behavior: every `run` target command ran through `sh -lc`, including deterministic benchmark fixtures.
- Current status: batch JSONL supports `primary_argv`, `candidate_argv`, and `secondary_argv`; `run` supports matching `--primary-argv`, `--candidate-argv`, and `--secondary-argv` flags.
- Expected impact: Medium for small fast commands where shell startup dominates the command body.
- Risk level: Medium. Shell strings are flexible and user-facing; argv execution should be additive rather than replacing the current interface.
- Metric to watch: `match`, `candidate-diff`, `moonlight` tool-comparison per-case latency, and direct process startup time.
- Acceptance criterion: batch JSONL supports an optional argv form for primary/candidate/secondary targets, while existing string command behavior remains backward compatible.

## 5. Single Writer Task For Batch

- Previous behavior: each completed batch run awaited `RunWriter::append`, which serialized access through a mutex around a buffered file.
- Current status: completed batch runs are sent to a dedicated writer task, so the polling loop can continue driving ready cases while storage writes are serialized by that task.
- Expected impact: Medium when `--jobs` is high and target commands complete quickly.
- Risk level: Low to medium. Ordering may remain completion-order unless the product requires input-order persistence.
- Metric to watch: `moonlight-cli batch --jobs N` suite latency across `N = 1, 4, 8, 16`.
- Acceptance criterion: batch writes through one writer task fed by a channel, preserves every record exactly once, and does not regress error handling or flush behavior.

## 6. Compact Output Mode For `run`

- Current behavior: `run` prints pretty JSON by default, while `batch --emit-runs` prints compact JSONL and `--quiet` suppresses output.
- Current status: `run --compact` is implemented, and `--quiet` suppresses stdout when both flags are passed.
- Expected impact: Low to medium for benchmark loops and machine consumers that parse `run` output.
- Risk level: Low. This can be additive with a `--compact` flag.
- Metric to watch: single-scenario benchmark latency for `match`, `candidate-diff`, and `status-regression`.
- Acceptance criterion: `run --compact` writes the same record to storage and prints compact JSON to stdout, with pretty output remaining the default.

## 7. Indexed Storage Option

- Current behavior: stats and latest-run views are recomputed by deserializing JSONL records.
- Expected impact: High for long-lived local stores and UI/admin reads.
- Risk level: High. An index introduces consistency, rebuild, and compatibility concerns.
- Metric to watch: read latency with 1k, 10k, and 100k stored runs.
- Acceptance criterion: an optional summary/index file accelerates stats and latest-run queries, can be rebuilt from JSONL, and never replaces JSONL as the source of truth.

## 8. JSON Diff Allocation Pass

- Current behavior: nested JSON diffing builds `BTreeSet` key collections and allocates path strings while walking objects and arrays.
- Expected impact: Unknown until profiling shows JSON diffing dominates.
- Risk level: Medium. Diff ordering and paths are user-visible.
- Metric to watch: `nested-json-diff`, `large-json-diff`, and focused `moonlight-core` compare benchmarks for deeply nested objects.
- Acceptance criterion: any allocation-reducing rewrite preserves deterministic diff ordering, exact paths, and current classification output while improving profiled large JSON diff cases.
