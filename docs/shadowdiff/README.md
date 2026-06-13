# Shadowdiff

Shadowdiff is a small Rust + React reimplementation of Diffy-style shadow traffic comparison. A client sends traffic to the proxy, the proxy forwards each request to a primary reference service, a candidate service, and optionally a secondary reference service, then returns only the primary response to the client.

The stored record captures request metadata, backend status codes, selected/redacted headers, body previews and hashes, latency, raw candidate diffs, reference noise, noise-filtered diffs, and classification.

## Why Primary And Secondary Exist

The candidate is the service being evaluated. The primary and secondary are two instances of the established reference behavior. If primary and secondary disagree on a field, header, status, or body area, that area is treated as reference noise. If primary and secondary match but the candidate differs, Shadowdiff marks it as suspicious.

This matters for naturally unstable values such as timestamps, IDs, randomized ordering, host-specific headers, and other nondeterministic responses.

## Safety

Shadowing duplicates traffic to multiple services. Non-idempotent requests can duplicate writes, send emails, enqueue jobs, charge accounts, or trigger other side effects. Use this MVP with read-only endpoints, isolated demo services, disposable environments, or backends that are explicitly safe to receive duplicate traffic.

Sensitive headers are redacted by default:

- `authorization`
- `cookie`
- `set-cookie`
- `x-api-key`

Large bodies are captured as a SHA-256 hash plus a preview. Full secrets should never be sent to the UI or logs.

## Run Locally

Start the three demo services:

```sh
cargo run -p shadowdiff-demo-services -- primary
cargo run -p shadowdiff-demo-services -- candidate
cargo run -p shadowdiff-demo-services -- secondary
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

Run a CLI comparison:

```sh
cargo run -p moonlight-cli -- run \
  --primary 'printf "{\"value\":42}\n"' \
  --candidate 'printf "{\"value\":43}\n"'
```

The CLI stores records in the same JSONL format as the HTTP proxy. Use `cargo run -p moonlight-cli -- requests` or `cargo run -p moonlight-cli -- stats` to inspect them.

## Send Sample Traffic

```sh
chmod +x scripts/shadowdiff-send-sample-traffic.sh
scripts/shadowdiff-send-sample-traffic.sh
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
docker compose -f docker-compose.shadowdiff.yml up
```

Run the optional Diffy comparison stack too:

```sh
docker compose -f docker-compose.shadowdiff.yml --profile reference up
```

This starts three Diffy instances alongside moonlight:

- Diffy A: outer comparator at `http://127.0.0.1:8880`, UI at `http://127.0.0.1:8888`
- Diffy B: primary reference for Diffy A at `http://127.0.0.1:8890`, UI at `http://127.0.0.1:8898`
- Diffy C: secondary reference for Diffy A at `http://127.0.0.1:8900`, UI at `http://127.0.0.1:8908`

Diffy B and Diffy C compare the demo primary, candidate, and secondary services directly. Diffy A uses Diffy B and Diffy C as its primary and secondary references, and uses moonlight (`moonlight-http`) as its candidate, so traffic through Diffy A compares moonlight behavior against two independent Diffy proxies.

Then send traffic to moonlight and Diffy A:

```sh
INCLUDE_DIFFY=1 scripts/shadowdiff-send-sample-traffic.sh
```

The Diffy services use the public `diffy/diffy` Docker image. They are intentionally optional because image availability and runtime behavior can vary by platform.

## Benchmark

Run the benchmark stack:

```sh
scripts/shadowdiff-benchmark.sh
```

This uses `docker-compose.shadowdiff-benchmark.yml` to run the Rust services from release-built containers and configures moonlight with `SHADOWDIFF_RESPONSE_MODE=primary_then_shadow`.

Diffy A is a correctness harness, not the latency baseline. Its proxy adds another fan-out layer, so the benchmark compares latency by sending measured traffic directly to:

- moonlight at `http://127.0.0.1:8080`
- Diffy B at `http://127.0.0.1:8890`
- Diffy C at `http://127.0.0.1:8900`

The runner also sends a smaller validation pass through Diffy A at `http://127.0.0.1:8880` so its UI can be used to inspect moonlight-vs-Diffy response differences.

Benchmark outputs:

- `data/shadowdiff/benchmark/latest.json`
- `data/shadowdiff/benchmark/latest.md`

Useful overrides:

```sh
BENCHMARK_REQUESTS=1200 BENCHMARK_CONCURRENCY=16 scripts/shadowdiff-benchmark.sh
DIFFY_IMAGE=diffy/diffy:latest scripts/shadowdiff-benchmark.sh
```

Results are most meaningful after warmup and with the release-built benchmark containers. The benchmark mode returns the primary response before shadow comparison storage completes, so in-flight comparison records can be lost if the process exits immediately.

## Admin API

- `GET /api/health`
- `GET /api/config`
- `GET /api/requests`
- `GET /api/requests/:id`
- `GET /api/stats`

All other routes are treated as proxy routes and forwarded to the configured backends.

## Configuration

Environment variables:

- `SHADOWDIFF_BIND_ADDR`, default `127.0.0.1:8080`
- `SHADOWDIFF_PRIMARY_URL`, default `http://127.0.0.1:3001`
- `SHADOWDIFF_CANDIDATE_URL`, default `http://127.0.0.1:3002`
- `SHADOWDIFF_SECONDARY_URL`, default `http://127.0.0.1:3003`
- `SHADOWDIFF_ENABLE_CANDIDATE`, default `true`
- `SHADOWDIFF_ENABLE_SECONDARY`, default `true`
- `SHADOWDIFF_RESPONSE_MODE`, default `wait_all`; use `primary_then_shadow` to return the primary response before candidate/secondary comparison finishes
- `SHADOWDIFF_MAX_BODY_CAPTURE_BYTES`, default `8192`
- `SHADOWDIFF_REDACT_HEADERS`, comma-separated
- `SHADOWDIFF_IGNORED_JSON_PATHS`, comma-separated
- `SHADOWDIFF_IGNORED_HEADERS`, comma-separated
- `SHADOWDIFF_STORAGE_PATH`, default `data/shadowdiff/requests.jsonl`

The exposed config also includes:

- `enable_candidate = true`
- `enable_secondary = true`
- `return_backend = "primary"`
- `response_mode = "wait_all"`
- `max_body_capture_bytes`
- `redact_headers`
- `ignored_json_paths`
- `ignored_headers`

## Implemented

- Axum proxy and admin API.
- Reqwest forwarding to primary, candidate, and optional secondary.
- Hop-by-hop request/response header stripping.
- JSONL append-only storage plus in-memory index.
- Body previews and SHA-256 hashes.
- JSON structural diff with simple JSON path tracking.
- Ignored JSON paths and ignored headers.
- Noise filtering using primary-secondary differences.
- React dashboard, request list, detail view, diff viewer, and config panel.
- CLI command comparison through `moonlight-cli`.
- Rust demo services and sample traffic generator.
- Optional Docker Compose profile with Diffy A comparing moonlight against Diffy B and Diffy C.
- Release-container benchmark profile and JSON/Markdown benchmark report generator.

## Left For Later

- SQLite persistence and pagination for large histories.
- Replay API.
- Richer JSONPath matching with wildcards.
- Order-insensitive array comparison for set-like arrays.
- Request and response body redaction rules beyond headers.
- Authentication for the admin API.
- Production deployment hardening and bounded storage retention.
