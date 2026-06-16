# Moonlight Benchmark

Generated: `2026-06-15T23:43:35.150416+00:00`

## Latency

| Target | Requests | Success | Errors | Req/s | p50 ms | p95 ms | p99 ms | Mean ms | Max ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| moonlight | 600 | 600 | 0 | 3425.75 | 1.88 | 4.09 | 11.03 | 2.25 | 14.47 |
| diffy_b | 600 | 600 | 0 | 143.30 | 19.28 | 454.95 | 481.21 | 55.65 | 508.00 |
| diffy_c | 600 | 600 | 0 | 144.17 | 14.05 | 503.73 | 535.45 | 55.31 | 561.86 |

## Status Codes

| Target | Status counts |
| --- | --- |
| moonlight | `{"200": 600}` |
| diffy_b | `{"200": 600}` |
| diffy_c | `{"200": 600}` |

## Direct Validity

| Endpoint | Result | Mismatches |
| --- | --- | --- |
| `/success` | match | - |
| `/regression` | match | - |
| `/noise` | match | - |
| `/noisy-regression` | match | - |
| `/status-regression` | match | - |
| `/slow-candidate` | match | - |

## Moonlight Stats

```json
{
  "latency": {
    "candidate_avg_ms": 110.61983059292476,
    "primary_avg_ms": 0.013452914798206279,
    "secondary_avg_ms": 0.013452914798206279
  },
  "latest_runs": [
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 400,
      "classification": "suspicious_difference",
      "diff_count": 3,
      "id": "dbf0189d-4965-4591-8cc6-9dfadea709d4",
      "input": {
        "method": "GET",
        "path": "/status-regression",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.088294136Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "8d6898c2-cf41-4784-bd14-91e4316f7ae9",
      "input": {
        "method": "GET",
        "path": "/noisy-regression",
        "query": null
      },
      "noise_count": 1,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.084354458Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "reference_noise",
      "diff_count": 0,
      "id": "78753b31-d0bb-410c-b3d7-61bc0355282d",
      "input": {
        "method": "GET",
        "path": "/noise",
        "query": null
      },
      "noise_count": 2,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.079648310Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_difference",
      "diff_count": 1,
      "id": "894a842a-019e-4ab4-8184-baf2e91fcd02",
      "input": {
        "method": "GET",
        "path": "/regression",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.073275617Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "match",
      "diff_count": 0,
      "id": "9b5e2015-ed81-45e1-ad99-4274d6cd44e2",
      "input": {
        "method": "GET",
        "path": "/success",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.061849544Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "bbc49e87-22fa-46dd-9d69-ded092cab6de",
      "input": {
        "method": "GET",
        "path": "/noisy-regression",
        "query": null
      },
      "noise_count": 1,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.059590096Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 400,
      "classification": "suspicious_difference",
      "diff_count": 3,
      "id": "480826c3-c97d-4c6d-bc65-541a00454596",
      "input": {
        "method": "GET",
        "path": "/status-regression",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 1,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.059009374Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "match",
      "diff_count": 0,
      "id": "7ccb51d0-1660-492a-9103-77ee4df0b43c",
      "input": {
        "method": "GET",
        "path": "/success",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.056291654Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_difference",
      "diff_count": 1,
      "id": "2c63eba8-1d17-497c-99a6-d9a94f3e526e",
      "input": {
        "method": "GET",
        "path": "/regression",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.051103228Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "reference_noise",
      "diff_count": 0,
      "id": "10a2f4b5-acd6-4c4c-add1-7302a4518234",
      "input": {
        "method": "GET",
        "path": "/noise",
        "query": null
      },
      "noise_count": 2,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.050241227Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_difference",
      "diff_count": 1,
      "id": "54d11fda-5e0c-4ab6-9307-41cf2153315f",
      "input": {
        "method": "GET",
        "path": "/regression",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.039805065Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "168b87a4-a188-4d94-979f-9244864a56e1",
      "input": {
        "method": "GET",
        "path": "/noisy-regression",
        "query": null
      },
      "noise_count": 1,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.028634181Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 400,
      "classification": "suspicious_difference",
      "diff_count": 3,
      "id": "51104a09-02d2-4d1c-bf8d-74bf0740cb93",
      "input": {
        "method": "GET",
        "path": "/status-regression",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.028354866Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "reference_noise",
      "diff_count": 0,
      "id": "fe24de03-747d-4215-8bcd-34c79ed91181",
      "input": {
        "method": "GET",
        "path": "/noise",
        "query": null
      },
      "noise_count": 2,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.015578593Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 400,
      "classification": "suspicious_difference",
      "diff_count": 3,
      "id": "34561562-e3e4-41bb-9112-1e29721260d7",
      "input": {
        "method": "GET",
        "path": "/status-regression",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:35.004164903Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "match",
      "diff_count": 0,
      "id": "de0c29a8-b7eb-410e-b327-3a4437c9a72e",
      "input": {
        "method": "GET",
        "path": "/success",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 2,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:34.990256863Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "f22fe828-b342-4d5c-b25f-0d7bb1ed3aa5",
      "input": {
        "method": "GET",
        "path": "/noisy-regression",
        "query": null
      },
      "noise_count": 1,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 2,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:34.985579589Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "reference_noise",
      "diff_count": 0,
      "id": "5b377e30-8c30-450a-9769-791ac10c93d4",
      "input": {
        "method": "GET",
        "path": "/noise",
        "query": null
      },
      "noise_count": 2,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:34.955383042Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_difference",
      "diff_count": 1,
      "id": "edd49119-4f66-4bbf-9bf0-9e5f63f40728",
      "input": {
        "method": "GET",
        "path": "/regression",
        "query": null
      },
      "noise_count": 0,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:34.654986879Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "cd7b6691-96f9-45b3-902a-44f8cc1d761f",
      "input": {
        "method": "GET",
        "path": "/noisy-regression",
        "query": null
      },
      "noise_count": 1,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 0,
      "secondary_status": 200,
      "timestamp": "2026-06-15T23:43:34.652845532Z"
    }
  ],
  "matches": 687,
  "reference_noise": 328,
  "suspicious_differences": 663,
  "suspicious_with_noise": 329,
  "target_errors": 0,
  "total_runs": 2007
}
```
