# Moonlight

Moonlight is a small Rust + React behavior comparer. A client sends traffic to the HTTP proxy, the proxy forwards each request to a primary reference target, a candidate target, and optionally a secondary reference target, then returns the configured target response to the client.

Each stored comparison run captures adapter input, target status codes, selected/redacted headers, body previews and hashes, stderr for CLI targets, latency, raw candidate diffs, reference noise, noise-filtered diffs, and classification.

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

Large bodies are captured as a SHA-256 hash plus a preview. Full secrets should never be sent to the UI or logs.

## Run Locally

Start the three demo services:

```sh
cargo run -p moonlight-demo-services -- primary
cargo run -p moonlight-demo-services -- candidate
cargo run -p moonlight-demo-services -- secondary
```

Start the proxy:

```sh
cargo run -p moonlight-http
```

Start the UI:

```sh
bun install
bun run dev
```

Open `http://127.0.0.1:5173`.

## GitHub Pages Example

The repository includes a static GitHub Pages example workflow at `.github/workflows/pages.yml`. It deploys the Vite UI with bundled demo comparison runs, so the page works without a live Moonlight admin API.

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

Run a CLI comparison:

```sh
cargo run -p moonlight-cli -- run \
  --primary 'printf "{\"value\":42}\n"' \
  --candidate 'printf "{\"value\":43}\n"'
```

`run` compares two or three target commands: primary, candidate, and optionally secondary. Targets run concurrently by default. Use `--serial-targets` when command order matters, and `--quiet` when only the stored JSONL record is needed:

```sh
cargo run -p moonlight-cli -- run \
  --primary 'printf primary\n' \
  --candidate 'printf candidate\n' \
  --serial-targets \
  --quiet
```

For trycmd-like command suites, use `batch` so many cases run inside one `moonlight-cli` process with bounded concurrency:

```sh
cat > cases.jsonl <<'JSONL'
{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":42}'"}
{"primary":"printf '%s\n' '{\"value\":42}'","candidate":"printf '%s\n' '{\"value\":43}'"}
JSONL

cargo run -p moonlight-cli -- batch --input cases.jsonl --jobs 8
```

Each JSONL case accepts `primary`, `candidate`, optional `secondary`, optional `max_body_capture_bytes`, optional `ignored_json_paths`, optional `ignored_headers`, and optional `ignore_stderr`. Use `--input -` to read cases from stdin, `--quiet` to suppress the summary, or `--emit-runs` to print compact JSONL run records as cases complete.

The CLI stores comparison runs in the same JSONL format as the HTTP proxy. Use `cargo run -p moonlight-cli -- list` or `cargo run -p moonlight-cli -- stats` to inspect them.

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

This uses `docker-compose.moonlight-benchmark.yml` to run the Rust services from release-built containers and configures moonlight with `MOONLIGHT_RESPONSE_TIMING=return_selected`.

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

- `data/moonlight/cli-benchmark/latest.json`
- `data/moonlight/cli-benchmark/latest.md`

The report also includes a tool comparison table for simple command-output checks:

- `moonlight`, measured through `moonlight-cli batch` with primary/candidate command cases.
- `trycmd`, measured through a generated throwaway Cargo test harness.
- `cram`, measured when a `cram` executable is available on `PATH`; otherwise it is reported as skipped.

`moonlight-cli batch` still compares primary and candidate behavior for every case, while `trycmd` generally checks one command against a stored stdout/stderr snapshot.

Useful overrides:

```sh
BENCHMARK_REQUESTS=500 BENCHMARK_CONCURRENCY=1 scripts/moonlight-cli-benchmark.sh
BENCHMARK_COMPARISON_RUNS=50 BENCHMARK_COMPARISON_CASES=100 scripts/moonlight-cli-benchmark.sh
python3 scripts/moonlight-cli-benchmark.py --bin target/release/moonlight-cli --scenario candidate-diff
python3 scripts/moonlight-cli-benchmark.py --target moonlight --target trycmd --target cram
```

## Admin API

- `GET /api/health`
- `GET /api/config`
- `GET /api/runs`
- `GET /api/runs/:id`
- `GET /api/stats`

All other routes are treated as proxy routes and forwarded to the configured targets.

## Configuration

Environment variables:

- `MOONLIGHT_BIND_ADDR`, default `127.0.0.1:8080`
- `MOONLIGHT_PRIMARY_URL`, default `http://127.0.0.1:3001`
- `MOONLIGHT_CANDIDATE_URL`, default `http://127.0.0.1:3002`
- `MOONLIGHT_SECONDARY_URL`, default `http://127.0.0.1:3003`
- `MOONLIGHT_ENABLE_SECONDARY`, default `true`
- `MOONLIGHT_RETURN_TARGET`, default `primary`; use `candidate` to return the candidate response
- `MOONLIGHT_RETURN_FALLBACK`, default `none`; use `primary` to fall back when candidate return has a target error
- `MOONLIGHT_RESPONSE_TIMING`, default `wait_all`; use `return_selected` to return the selected response before remaining comparison work finishes
- `MOONLIGHT_MAX_BODY_CAPTURE_BYTES`, default `8192`
- `MOONLIGHT_REDACT_HEADERS`, comma-separated
- `MOONLIGHT_IGNORED_JSON_PATHS`, comma-separated
- `MOONLIGHT_IGNORED_HEADERS`, comma-separated
- `MOONLIGHT_IGNORE_STDERR`, default `false`
- `MOONLIGHT_STORAGE_PATH`, default `data/moonlight/http-runs.jsonl`

The exposed config also includes:

- `enable_secondary = true`
- `return_target = "primary"`
- `return_fallback = "none"`
- `response_timing = "wait_all"`
- `max_body_capture_bytes`
- `redact_headers`
- `ignored_json_paths`
- `ignored_headers`
- `ignore_stderr`

## Implemented

- Axum proxy and admin API.
- Reqwest forwarding to primary, candidate, and optional secondary.
- Hop-by-hop request/response header stripping.
- JSONL append-only storage plus in-memory index over `data/moonlight/*.jsonl`.
- Body previews and SHA-256 hashes.
- JSON structural diff with simple JSON path tracking.
- Ignored JSON paths and ignored headers.
- Noise filtering using primary-secondary differences and candidate-must-match-reference semantics.
- React dashboard, run list, detail view, diff viewer, and config panel.
- CLI command comparison through `moonlight-cli`.
- Rust demo services and sample traffic generator.
- Optional Docker Compose profile with Diffy A comparing Moonlight against Diffy B and Diffy C.
- Release-container benchmark profile and JSON/Markdown benchmark report generator.

## Left For Later

- SQLite persistence and pagination for large histories.
- Replay API.
- Richer JSONPath matching with wildcards.
- Order-insensitive array comparison for set-like arrays.
- Request and response body redaction rules beyond headers.
- Authentication for the admin API.
- Production deployment hardening and bounded storage retention.
