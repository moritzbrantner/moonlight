# moonlight-cli Benchmark

Generated: `2026-06-13T22:22:38.468250+00:00`

## Scenarios

| Scenario | Invocations | Success | Errors | Records | p50 ms | p95 ms | p99 ms | Mean ms | Max ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| match | 200 | 200 | 0 | 200 | 21.29 | 25.21 | 27.26 | 21.56 | 29.15 |
| candidate-diff | 200 | 200 | 0 | 200 | 17.97 | 19.24 | 19.57 | 18.02 | 19.92 |
| noise | 200 | 200 | 0 | 200 | 19.12 | 22.25 | 24.74 | 19.50 | 28.76 |
| noisy-regression | 200 | 200 | 0 | 200 | 20.04 | 27.43 | 32.78 | 21.06 | 41.00 |
| status-regression | 200 | 200 | 0 | 200 | 18.18 | 20.54 | 23.99 | 18.39 | 26.27 |
| stderr-diff | 200 | 200 | 0 | 200 | 17.88 | 19.30 | 19.93 | 18.00 | 21.08 |
| large-body | 200 | 200 | 0 | 200 | 49.16 | 58.20 | 65.22 | 50.10 | 68.27 |
| nested-json-diff | 200 | 200 | 0 | 200 | 18.77 | 20.75 | 23.74 | 18.95 | 30.33 |
| ignored-dynamic-json | 200 | 200 | 0 | 200 | 19.56 | 25.50 | 28.97 | 20.49 | 30.48 |
| large-json-match | 200 | 200 | 0 | 200 | 46.58 | 52.97 | 60.15 | 47.51 | 65.84 |
| large-json-diff | 200 | 200 | 0 | 200 | 54.02 | 83.98 | 104.40 | 56.93 | 105.78 |
| large-stderr-match | 200 | 200 | 0 | 200 | 47.49 | 67.38 | 105.12 | 51.31 | 140.35 |
| serial-targets | 200 | 200 | 0 | 200 | 33.40 | 39.82 | 44.53 | 33.98 | 48.58 |
| truncated-capture | 200 | 200 | 0 | 200 | 47.74 | 84.85 | 102.47 | 52.67 | 147.19 |

## Classifications

| Scenario | Counts |
| --- | --- |
| match | `{"match": 200}` |
| candidate-diff | `{"suspicious_difference": 200}` |
| noise | `{"reference_noise": 200}` |
| noisy-regression | `{"suspicious_with_noise": 200}` |
| status-regression | `{"suspicious_difference": 200}` |
| stderr-diff | `{"suspicious_difference": 200}` |
| large-body | `{"match": 200}` |
| nested-json-diff | `{"suspicious_difference": 200}` |
| ignored-dynamic-json | `{"match": 200}` |
| large-json-match | `{"match": 200}` |
| large-json-diff | `{"suspicious_difference": 200}` |
| large-stderr-match | `{"match": 200}` |
| serial-targets | `{"match": 200}` |
| truncated-capture | `{"match": 200}` |

## Tool Comparisons

Each comparison case is a deterministic shell command-output check; moonlight runs a primary/candidate comparison, while the other targets run snapshot-style assertions when available.

| Target | Status | Suite Runs | Cases/Run | Target invocations/case | Total Cases | Total Target Invocations | Suite p50 ms | Suite p95 ms | Per-case p50 ms | Per-case p95 ms | Per-target p50 ms | Per-target p95 ms | Version/Reason |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| moonlight | ok | 20 | 25 | 2 | 500 | 1000 | 108.00 | 116.54 | 4.32 | 4.66 | 2.16 | 2.33 | target/release/moonlight-cli batch |
| moonlight-argv | ok | 20 | 25 | 2 | 500 | 1000 | 30.48 | 37.57 | 1.22 | 1.50 | 0.61 | 0.75 | target/release/moonlight-cli batch argv |
| trycmd | ok | 20 | 25 | 1 | 500 | 500 | 66.46 | 72.33 | 2.66 | 2.89 | 2.66 | 2.89 | trycmd 1.2.0 via cargo 1.96.0 (30a34c682 2026-05-25) |
| insta | ok | 20 | 25 | 1 | 500 | 500 | 403.46 | 441.55 | 16.14 | 17.66 | 16.14 | 17.66 | insta 1.48.0 via cargo 1.96.0 (30a34c682 2026-05-25) |
| cram | ok | 20 | 25 | 1 | 500 | 500 | 44.09 | 45.19 | 1.76 | 1.81 | 1.76 | 1.81 | Cram CLI testing framework (version 0.7) |
| bats | skipped | 0 | 25 | 1 | 0 | 0 | - | - | - | - | - | - | bats executable not found on PATH |
| shellspec | skipped | 0 | 25 | 1 | 0 | 0 | - | - | - | - | - | - | shellspec executable not found on PATH |

## Validation

All validations passed.
