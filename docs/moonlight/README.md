# Moonlight

Moonlight is a small Rust + React behavior comparer. A client sends traffic to the HTTP proxy, the proxy forwards each request to a primary reference target, a candidate target, and optionally a secondary reference target, then returns the configured target response to the client.

Each stored comparison run captures adapter input, target status codes, selected/redacted headers, body previews and hashes, stderr for CLI targets, latency, raw candidate diffs, reference noise, noise-filtered diffs, and classification.

GitHub Pages overview and latest benchmark snapshot: <https://moritzbrantner.github.io/moonlight/?page=overview>

## Why Primary And Secondary Exist

The candidate is the target being evaluated. The primary and secondary references are two instances of the established reference behavior. If primary and secondary disagree on a field, header, status, stderr stream, or body area, that area is treated as reference noise. Candidate behavior is only filtered on a noisy field when it matches one of the references; if it differs from both references, Moonlight marks it as a suspicious difference.

This matters for naturally unstable values such as timestamps, IDs, randomized ordering, host-specific headers, and other nondeterministic responses.

## Safety

Shadowing duplicates traffic to multiple services. Non-idempotent requests can duplicate writes, send emails, enqueue jobs, charge accounts, or trigger other side effects. Use this MVP with read-only endpoints, isolated demo services, disposable environments, or targets that are explicitly safe to receive duplicate traffic.

Sensitive headers are redacted by default:

- `authorization`
- `cookie`
- `set-cookie`
- `x-api-key`
- `proxy-authorization`
- `x-auth-token`
- `x-csrf-token`

Large bodies are captured as a SHA-256 hash plus a preview. Configure
`[comparison].redact_json_paths` or `[comparison].redact_json_path_patterns` in
`moonlight.conf`, or pass `--redact-json-path` or
`--redact-json-path-pattern`, to redact JSON body paths from stored previews and
diff values. Hashes still represent the original body bytes, and non-JSON
bodies are not rewritten by JSON path redaction.

Moonlight is not a data-loss-prevention system. Full secrets should never be
sent to the UI, logs, or comparison storage, and sensitive write traffic should
not be shadowed.

## Install The CLI

Install with Cargo:

```sh
cargo install moonlight-cli --locked
moonlight run --primary 'printf "{\"value\":42}\n"' --candidate 'printf "{\"value\":43}\n"'
```

Run through npm:

```sh
npx @moritzbrantner/moonlight run \
  --primary 'printf "{\"value\":42}\n"' \
  --candidate 'printf "{\"value\":43}\n"'
```

Run through Bun:

```sh
bunx @moritzbrantner/moonlight run \
  --primary 'printf "{\"value\":42}\n"' \
  --candidate 'printf "{\"value\":43}\n"'
```

The installed commands are `moonlight` and `moonlight-cli`. They run the same
CLI; `moonlight-cli` is kept as a compatibility alias.

## Run Locally

Start the three demo services:

```sh
cargo run -p moonlight-demo-services -- primary
cargo run -p moonlight-demo-services -- candidate
cargo run -p moonlight-demo-services -- secondary
```

Start the proxy:

```sh
cargo run -p moonlight-http -- \
  --primary-url http://127.0.0.1:3001 \
  --candidate-url http://127.0.0.1:3002 \
  --secondary-url http://127.0.0.1:3003
```

`moonlight-http` emits structured logs through `tracing`. Set `RUST_LOG` to
adjust verbosity, for example `RUST_LOG=moonlight_http=debug,tower_http=info`.
Without `RUST_LOG`, the default filter is
`moonlight_http=info,moonlight_core=info,tower_http=info`.

Pass flags to override `moonlight.conf` when comparing local services:

```sh
cargo run -p moonlight-http -- \
  --primary-url http://127.0.0.1:3001 \
  --candidate-url http://127.0.0.1:3002 \
  --return-target primary \
  --storage-path data/moonlight/json-server-runs.jsonl
```

Start the UI:

```sh
bun install
bun run dev
```

Open `http://127.0.0.1:5173`.

Run the UI component test suite:

```sh
bun run test:run
```

Review components in Storybook:

```sh
bun run storybook
bun run storybook:build
```

Storybook includes the accessibility addon, so component stories expose local WCAG A/AA checks while reviewing UI changes.

## GitHub Pages Example

The repository includes a static GitHub Pages example workflow at `.github/workflows/pages.yml`. It deploys the Vite UI with a repository overview page, latest HTTP and CLI benchmark snapshots, and bundled demo comparison runs, so the page works without a live Moonlight admin API.

The workflow uses the shared Pages deployment workflow from `moritzbrantner/reusable-workflows`:

```yaml
jobs:
  deploy-pages:
    uses: moritzbrantner/reusable-workflows/.github/workflows/deploy-pages.yml@workflow-standard-v1.2
    with:
      node_version: "24"
      bun_version: "1.3.14"
      install_command: bun install --frozen-lockfile
      build_command: VITE_MOONLIGHT_DEMO=true VITE_MOONLIGHT_BASE_PATH=/moonlight/ bun run build
      artifact_path: apps/moonlight-ui/dist
      timeout_minutes: 10
      bun_cache_dependency_path: bun.lock
```

For a different repository name, change `VITE_MOONLIGHT_BASE_PATH` to the repository path used by GitHub Pages. To deploy against a live API instead of the bundled example data, remove `VITE_MOONLIGHT_DEMO=true` and set `VITE_MOONLIGHT_API_URL` to the public admin API origin.

Run an installed CLI comparison:

```sh
moonlight run \
  --primary 'printf "{\"value\":42}\n"' \
  --candidate 'printf "{\"value\":43}\n"'
```

For contributor builds from this repository, use `cargo run -p moonlight-cli --`
before the subcommand.

`run` compares two or three target commands: primary, candidate, and optionally secondary. Targets run concurrently by default. Use `--serial-targets` when command order matters, `--compact` when a machine consumer wants one-line JSON on stdout, and `--quiet` when only the stored JSONL record is needed:

```sh
moonlight run \
  --primary 'printf primary\n' \
  --candidate 'printf candidate\n' \
  --serial-targets \
  --quiet
```

Trusted deterministic `run` commands can use direct argv flags to avoid shell startup and parsing:

```sh
moonlight run \
  --primary-argv '["printf","%s\n","{\"value\":42}"]' \
  --candidate-argv '["printf","%s\n","{\"value\":43}"]' \
  --compact
```

For each required target role, provide exactly one shell string flag or argv flag: `--primary` or `--primary-argv`, and `--candidate` or `--candidate-argv`. For the optional secondary target, provide at most one of `--secondary` or `--secondary-argv`. Argv values must be JSON string arrays with a nonblank executable as the first element. Stored CLI run input remains backward compatible by recording the argv command as a shell-escaped display string.

For trycmd-like command suites, use `batch` so many cases run inside one
`moonlight` process with bounded concurrency:

```sh
cat > cases.jsonl <<'JSONL'
{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":42}'"}
{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":43}'"}
JSONL

moonlight batch --input cases.jsonl --jobs 8
```

Each JSONL case accepts `primary`, `candidate`, optional `secondary`, optional
`max_body_capture_bytes`, optional `ignore_json_paths`, optional
`ignore_headers`, and optional `ignore_stderr`. Shell string commands remain
the default and run through `sh -lc`.

Trusted deterministic batch fixtures can use direct argv fields to avoid shell startup and parsing:

```json
{"primary_argv":["printf","%s\n","{\"value\":42}"],"candidate_argv":["printf","%s\n","{\"value\":42}"]}
```

For each target role, provide exactly one form: `primary` or `primary_argv`, `candidate` or `candidate_argv`, and optionally `secondary` or `secondary_argv`. Argv arrays must be non-empty and start with a nonblank executable. Stored CLI run input remains backward compatible by recording a display string for argv commands.

Use `--input -` to read cases from stdin, `--quiet` to suppress the summary, or
`--emit-runs` to print compact JSONL run records as cases complete. For
deterministic command-output suites, prefer `batch` with `*_argv` fields because
it avoids one `moonlight` process per case and bypasses shell startup for each
target.

The CLI stores comparison runs in the same JSONL format as the HTTP proxy. Use `moonlight list` or `moonlight stats` to inspect them:

```sh
moonlight list --limit 20
moonlight list --limit 20 --offset 20 --compact
moonlight stats --compact
moonlight show "$RUN_ID" --compact
```

`moonlight ls` is an alias for `moonlight list`. CLI read commands inspect only
the requested JSONL file, so nearby HTTP or benchmark JSONL files do not affect
`list`, `stats`, or `show`. Set `[storage].path` in `moonlight.conf` to change
the default storage file; an explicit `--storage-path` flag always wins.

CLI non-zero exit statuses and HTTP 4xx/5xx response statuses are observed target statuses, so Moonlight compares and stores them like other target output. `target_error` is reserved for invocation or capture failures such as spawn, read, wait, signal, transport, or body-read failures that prevent a complete target observation.

### Install Troubleshooting

If npm reports that the optional native package is missing, reinstall with
optional dependencies enabled. npm users should avoid `--omit=optional`; Bun
users should retry with a clean install cache if `bunx` reused an incomplete
download.

If your platform is unsupported by the npm package, install with Cargo instead:

```sh
cargo install moonlight-cli --locked
```

Set `MOONLIGHT_BIN=/path/to/moonlight` to make the npm launcher use a local or
manually installed binary.

## Send Sample Traffic

```sh
chmod +x scripts/moonlight-send-sample-traffic.sh
scripts/moonlight-send-sample-traffic.sh
```

The script hits:

- `/success`
- `/regression`
- `/noise`
- `/noisy-regression`
- `/status-regression`
- `/slow-candidate`

Use `ROUNDS=10` to send more traffic.

## Docker Compose

Run the demo stack:

```sh
docker compose -f docker-compose.moonlight.yml up
```

Run the optional Diffy comparison stack too:

```sh
docker compose -f docker-compose.moonlight.yml --profile reference up
```

This starts three Diffy instances alongside moonlight:

- Diffy A: outer comparator at `http://127.0.0.1:8880`, UI at `http://127.0.0.1:8888`
- Diffy B: primary reference for Diffy A at `http://127.0.0.1:8890`, UI at `http://127.0.0.1:8898`
- Diffy C: secondary reference for Diffy A at `http://127.0.0.1:8900`, UI at `http://127.0.0.1:8908`

Diffy B and Diffy C compare the demo primary, candidate, and secondary services directly. Diffy A uses Diffy B and Diffy C as its primary and secondary references, and uses moonlight (`moonlight-http`) as its candidate, so traffic through Diffy A compares moonlight behavior against two independent Diffy proxies.

Then send traffic to moonlight and Diffy A:

```sh
INCLUDE_DIFFY=1 scripts/moonlight-send-sample-traffic.sh
```

The Diffy services use the public `diffy/diffy` Docker image. They are intentionally optional because image availability and runtime behavior can vary by platform.

## Benchmark

Run the benchmark stack:

```sh
scripts/moonlight-benchmark.sh
```

This uses `docker-compose.moonlight-benchmark.yml` to run the Rust services
from release-built containers and starts moonlight with
`--response-timing return-selected`.

Diffy A is a correctness harness, not the latency baseline. Its proxy adds another fan-out layer, so the benchmark compares latency by sending measured traffic directly to:

- moonlight at `http://127.0.0.1:8080`
- Diffy B at `http://127.0.0.1:8890`
- Diffy C at `http://127.0.0.1:8900`

The runner also sends a smaller validation pass through Diffy A at `http://127.0.0.1:8880` so its UI can be used to inspect moonlight-vs-Diffy response differences.

Benchmark outputs:

- `data/moonlight/benchmark/latest.json`
- `data/moonlight/benchmark/latest.md`

Useful overrides:

```sh
BENCHMARK_REQUESTS=1200 BENCHMARK_CONCURRENCY=16 scripts/moonlight-benchmark.sh
DIFFY_IMAGE=diffy/diffy:latest scripts/moonlight-benchmark.sh
```

Results are most meaningful after warmup and with the release-built benchmark containers. The benchmark mode returns the selected response before comparison storage completes, so in-flight comparison runs can be lost if the process exits immediately.

## Project Eval For Coding Agents

Moonlight can evaluate a coding-agent patch against an existing Git project by
creating isolated local worktrees, running the same checks against the baseline
and candidate, and storing one project comparison run per check.

Create `moonlight.eval.toml` in the project you want to evaluate:

```toml
[project]
name = "my-project"
repo = "."
baseline_ref = "main"

[eval]
work_dir = ".moonlight/evals"
keep_worktrees = "failed"
jobs = 2
target_timeout_ms = 600000
max_body_capture_bytes = 20000

[[checks]]
id = "test"
command = "cargo test --workspace"
cwd = "."
timeout_ms = 600000

[[checks]]
id = "lint"
argv = ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]
cwd = "."
timeout_ms = 600000
```

Then evaluate a patch produced by a coding agent:

```sh
git diff --binary main > agent.patch
moonlight eval run --candidate-patch agent.patch --format markdown
```

Or compare a candidate branch or commit:

```sh
moonlight eval run --candidate-ref agent/output-branch
```

`moonlight eval run` exits `0` when every check matches or only contains
reference noise, exits `1` when a check shows a suspicious difference or target
error, and exits `2` for setup failures such as invalid config, git worktree
errors, or a patch that does not apply.

Each `[[checks]]` entry accepts exactly one of `command` or `argv`, plus
optional `cwd`, `env`, `timeout_ms`, `ignore_stdout`, `ignore_stderr`,
`normalize_stdout_patterns`, and `normalize_stderr_patterns`. Normalization
patterns are Rust regular expressions replaced with `<normalized>` before
output comparison and storage.

By default, eval worktrees are removed after successful runs and kept after
failed runs for debugging. Use `--keep-worktrees never`, `failed`, or `always`
to override the config. Use `moonlight eval report <EVAL_ID> --format markdown`
to render a summary from stored JSONL runs.

## CLI Testing And Benchmarking

Run the direct `moonlight-cli` test suite:

```sh
cargo test -p moonlight-cli
```

Run the full Rust workspace suite:

```sh
cargo test --workspace
```

Run the Criterion microbenchmarks for deterministic CLI scenarios:

```sh
cargo bench -p moonlight-cli --bench moonlight_cli
```

Run the scenario benchmark runner:

```sh
scripts/moonlight-cli-benchmark.sh
```

The scenario runner builds the release CLI binary, invokes deterministic local command comparisons, validates the JSONL records and classification counts, and writes reports to:

- `data/moonlight/cli-benchmark-analysis/latest.json`
- `data/moonlight/cli-benchmark-analysis/latest.md`

The report also includes a tool comparison table for simple command-output checks:

- `moonlight`, measured through `moonlight batch` with primary/candidate command cases.
- `moonlight-argv`, measured through `moonlight batch` with direct argv primary/candidate cases.
- `trycmd`, measured through a generated throwaway Cargo test harness.
- `insta`, measured through a generated throwaway Cargo test harness with inline snapshots.
- `cram`, measured when a `cram` executable is available on `PATH`; otherwise it is reported as skipped.
- `bats`, measured when a `bats` executable is available on `PATH`; otherwise it is reported as skipped.
- `shellspec`, measured when a `shellspec` executable is available on `PATH`; otherwise it is reported as skipped.

`moonlight batch` still compares primary and candidate behavior for every case,
while snapshot-style targets generally check one command against stored
stdout/stderr expectations. The report keeps raw per-case latency and also
includes normalized per-target latency columns so Moonlight's two target
invocations per case are visible.

Performance improvement candidates for the CLI and shared core are tracked in [`cli-performance-ideas.md`](cli-performance-ideas.md).

Useful overrides:

```sh
BENCHMARK_REQUESTS=500 BENCHMARK_CONCURRENCY=1 scripts/moonlight-cli-benchmark.sh
BENCHMARK_COMPARISON_RUNS=50 BENCHMARK_COMPARISON_CASES=100 scripts/moonlight-cli-benchmark.sh
python3 scripts/moonlight-cli-benchmark.py --bin target/release/moonlight --scenario candidate-diff
python3 scripts/moonlight-cli-benchmark.py --bin target/release/moonlight --baseline-bin /path/to/old/moonlight --target moonlight --target moonlight-argv
python3 scripts/moonlight-cli-benchmark.py --target moonlight --target moonlight-argv --target trycmd --target insta --target cram --target bats --target shellspec
```

## UI Testing And Benchmarking

Run the Vitest and React Testing Library component suite:

```sh
bun run test:run
```

Build the production UI and Storybook:

```sh
bun run build
bun run storybook:build
```

Run the static demo UI benchmark used by CI:

```sh
scripts/ui-unlighthouse.sh
```

This builds the Vite UI with `VITE_MOONLIGHT_DEMO=true`, serves the production build locally, and runs Unlighthouse against the overview and dashboard routes. Reports are written to:

- `performance-results/unlighthouse`
- `performance-results/unlighthouse-summary.md`

Run an optional benchmark against a live UI and admin API:

```sh
bun run dev
MOONLIGHT_UI_URL=http://127.0.0.1:5173 scripts/ui-unlighthouse-live.sh
```

The live benchmark is intended for local or manual runs because it depends on a running UI and API. Required CI benchmarks only the static demo UI through `moritzbrantner/reusable-workflows` performance validation.

## Admin API

- `GET /api/health`
- `GET /api/config`
- `GET /api/runs?limit=100&offset=0`
- `GET /api/runs/:id`
- `GET /api/stats`
- `GET /api/metrics`

All other routes are treated as proxy routes and forwarded to the configured targets.

`GET /api/metrics` returns in-process counters for proxied comparisons,
persisted comparisons, persistence failures, storage refresh failures, target
errors, and classifications. Counters reset when `moonlight-http` restarts.

`GET /api/runs` returns a JSON array for backwards compatibility. `limit`
defaults to `100` and is capped at `1000`; `offset` defaults to `0`.

Set `[http].admin_token` in `moonlight.conf` or pass `--admin-token` to require
admin requests, except `GET /api/health`, to include either
`Authorization: Bearer <token>` or `X-Moonlight-Admin-Token: <token>`. Proxy
fallback routes remain unauthenticated because they represent the traffic under
comparison. Avoid committing config files with real tokens; passing tokens as
flags can also expose them through shell history or process listings.

## Configuration

`moonlight` and `moonlight-http` automatically read `./moonlight.conf` when it
exists. Use `--config <PATH>` to read a different TOML file or `--no-config` to
skip config discovery. Built-in defaults are applied first, then
`moonlight.conf`, then explicit CLI flags.

Example `moonlight.conf`:

```toml
[storage]
path = "data/moonlight/runs.jsonl"

[comparison]
max_body_capture_bytes = 8192
ignore_json_paths = ["$.timestamp", "$.requestId"]
ignore_headers = ["date", "x-request-id"]
ignore_stderr = false
redact_headers = ["x-internal-secret"]
redact_json_paths = ["$.token"]
redact_query_params = ["token"]

[cli.run]
primary = "printf '{\"value\":42}\n'"
candidate = "printf '{\"value\":43}\n'"
quiet = false
compact = false
serial_targets = false

[cli.batch]
input = "-"
jobs = 8
quiet = false
emit_runs = false
serial_targets = false

[http]
bind_addr = "127.0.0.1:8080"
primary_url = "http://127.0.0.1:3001"
candidate_url = "http://127.0.0.1:3002"
secondary_url = "http://127.0.0.1:3003"
return_target = "primary"
return_fallback = "none"
response_timing = "wait-all"
max_request_body_bytes = 10485760
cors_origins = ["http://127.0.0.1:5173"]
retention_max_runs = 1000
retention_max_bytes = 104857600
```

List settings extend built-in safe defaults. For example, configured
`redact_headers` add to the default sensitive header redaction list, and
repeatable flags such as `--ignore-header date --ignore-header server` add more
comparison filters.

`moonlight-http` requires `primary_url` and `candidate_url` from config or
flags. The secondary reference is disabled by default and is enabled when
`secondary_url` is configured or `--enable-secondary` is passed.

Flags can override config:

```sh
cargo run -p moonlight-http -- \
  --primary-url http://127.0.0.1:3001 \
  --candidate-url http://127.0.0.1:3002 \
  --return-target primary \
  --storage-path data/moonlight/json-server-runs.jsonl
```

The exposed config also includes:

- `enable_secondary = true`
- `return_target = "primary"`
- `return_fallback = "none"`
- `response_timing = "wait_all"`
- `max_body_capture_bytes`
- `max_request_body_bytes`
- `redact_headers`
- `redact_json_paths`
- `redact_json_path_patterns`
- `redact_query_params`
- `ignore_json_paths`
- `ignore_json_path_patterns`
- `ignore_headers`
- `ignore_stderr`
- `cors_origins`
- `retention_max_runs`
- `retention_max_bytes`

`admin_token` is intentionally omitted from `GET /api/config`.

## Implemented

- Axum proxy and admin API.
- Reqwest forwarding to primary, candidate, and optional secondary.
- Hop-by-hop request/response header stripping.
- JSONL append-only storage plus in-memory index over `data/moonlight/*.jsonl`.
- Body previews and SHA-256 hashes.
- Optional exact JSON body path redaction for stored previews and diff values.
- Query parameter redaction for stored HTTP run input.
- JSON structural diff with simple JSON path tracking.
- Ignored JSON paths and ignored headers.
- Noise filtering using primary-secondary differences and candidate-must-match-reference semantics.
- React dashboard, run list, detail view, diff viewer, and config panel.
- CLI command comparison through `moonlight`.
- Project eval for checking candidate refs or patches against baseline Git worktrees.
- Rust demo services and sample traffic generator.
- Optional Docker Compose profile with Diffy A comparing Moonlight against Diffy B and Diffy C.
- Release-container benchmark profile and JSON/Markdown benchmark report generator.

## Left For Later

- SQLite persistence and pagination for large histories.
- Replay API.
- Richer JSONPath matching with wildcards.
- Order-insensitive array comparison for set-like arrays.
- Production deployment hardening beyond the local-first admin token and CORS controls.
