# moonlight-cli Benchmark

Generated: `2026-06-13T16:44:51.130686+00:00`

## Scenarios

| Scenario | Invocations | Success | Errors | Records | p50 ms | p95 ms | p99 ms | Mean ms | Max ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| match | 1 | 1 | 0 | 1 | 19.60 | 19.60 | 19.60 | 19.60 | 19.60 |

## Classifications

| Scenario | Counts |
| --- | --- |
| match | `{"match": 1}` |

## Tool Comparisons

Each comparison case is a deterministic shell command-output check; moonlight runs a primary/candidate comparison, while cram and trycmd run snapshot-style assertions when available.

| Target | Status | Suite Runs | Cases/Run | Total Cases | Suite p50 ms | Suite p95 ms | Per-case p50 ms | Per-case p95 ms | Version/Reason |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| moonlight | ok | 8 | 100 | 800 | 361.61 | 375.66 | 3.62 | 3.76 | target/release/moonlight-cli batch |
| trycmd | ok | 8 | 100 | 800 | 186.55 | 187.98 | 1.87 | 1.88 | trycmd 1.2.0 via cargo 1.96.0 (30a34c682 2026-05-25) |

## Validation

All validations passed.
