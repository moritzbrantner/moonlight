# moonlight-cli Benchmark

Generated: `2026-06-15T23:40:32.843853+00:00`

## Scenarios

| Scenario | Invocations | Success | Errors | Records | p50 ms | p95 ms | p99 ms | Mean ms | Max ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| match | 200 | 200 | 0 | 200 | 20.33 | 23.66 | 26.57 | 20.69 | 30.26 |
| match-argv | 200 | 200 | 0 | 200 | 3.86 | 4.79 | 6.36 | 4.00 | 6.53 |
| candidate-diff | 200 | 200 | 0 | 200 | 19.18 | 21.26 | 23.61 | 19.46 | 32.51 |
| noise | 200 | 200 | 0 | 200 | 19.61 | 21.10 | 22.54 | 19.75 | 28.44 |
| noisy-regression | 200 | 200 | 0 | 200 | 19.54 | 21.49 | 22.24 | 19.71 | 24.52 |
| status-regression | 200 | 200 | 0 | 200 | 18.69 | 20.67 | 23.10 | 18.91 | 25.30 |
| stderr-diff | 200 | 200 | 0 | 200 | 20.85 | 27.83 | 33.33 | 21.20 | 35.02 |
| large-body | 200 | 200 | 0 | 200 | 48.40 | 55.67 | 60.96 | 49.42 | 65.21 |
| nested-json-diff | 200 | 200 | 0 | 200 | 18.69 | 20.22 | 21.13 | 18.81 | 24.52 |
| ignored-dynamic-json | 200 | 200 | 0 | 200 | 18.74 | 20.67 | 26.80 | 18.98 | 31.75 |
| large-json-match | 200 | 200 | 0 | 200 | 46.79 | 50.89 | 57.74 | 47.42 | 64.65 |
| large-json-diff | 200 | 200 | 0 | 200 | 50.31 | 58.25 | 64.56 | 51.45 | 70.83 |
| large-stderr-match | 200 | 200 | 0 | 200 | 46.17 | 52.00 | 55.69 | 46.99 | 57.37 |
| serial-targets | 200 | 200 | 0 | 200 | 33.35 | 37.61 | 41.34 | 33.84 | 47.89 |
| truncated-capture | 200 | 200 | 0 | 200 | 47.06 | 55.42 | 60.46 | 48.39 | 101.70 |

## Classifications

| Scenario | Counts |
| --- | --- |
| match | `{"match": 200}` |
| match-argv | `{"match": 200}` |
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
| moonlight | ok | 20 | 25 | 2 | 500 | 1000 | 111.24 | 123.92 | 4.45 | 4.96 | 2.22 | 2.48 | target/release/moonlight-cli batch |
| moonlight-argv | ok | 20 | 25 | 2 | 500 | 1000 | 26.24 | 32.24 | 1.05 | 1.29 | 0.52 | 0.64 | target/release/moonlight-cli batch argv |
| trycmd | ok | 20 | 25 | 1 | 500 | 500 | 62.10 | 71.46 | 2.48 | 2.86 | 2.48 | 2.86 | trycmd 1.2.0 via cargo 1.96.0 (30a34c682 2026-05-25) |
| insta | ok | 20 | 25 | 1 | 500 | 500 | 410.00 | 435.22 | 16.40 | 17.41 | 16.40 | 17.41 | insta 1.48.0 via cargo 1.96.0 (30a34c682 2026-05-25) |
| cram | ok | 20 | 25 | 1 | 500 | 500 | 44.49 | 50.07 | 1.78 | 2.00 | 1.78 | 2.00 | Cram CLI testing framework (version 0.7) |
| bats | skipped | 0 | 25 | 1 | 0 | 0 | - | - | - | - | - | - | bats executable not found on PATH |
| shellspec | skipped | 0 | 25 | 1 | 0 | 0 | - | - | - | - | - | - | shellspec executable not found on PATH |

## Validation

All validations passed.
