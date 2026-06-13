# Moonlight Benchmark

Generated: `2026-06-13T14:10:48.506095+00:00`

## Latency

| Target | Requests | Success | Errors | Req/s | p50 ms | p95 ms | p99 ms | Mean ms | Max ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| moonlight | 600 | 600 | 0 | 3511.84 | 1.76 | 2.97 | 15.94 | 2.20 | 16.91 |
| diffy_b | 600 | 600 | 0 | 134.71 | 18.52 | 301.91 | 323.47 | 59.12 | 348.40 |
| diffy_c | 600 | 600 | 0 | 144.89 | 13.95 | 490.42 | 534.24 | 55.12 | 569.68 |

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
    "candidate_avg_ms": 112.53023255813953,
    "primary_avg_ms": 0.01317829457364341,
    "secondary_avg_ms": 0.008527131782945736
  },
  "latest_runs": [
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 400,
      "classification": "suspicious_difference",
      "diff_count": 3,
      "id": "732df90a-0905-455d-adfd-74e7a18502ca",
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
      "timestamp": "2026-06-13T14:10:48.464106020Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "1292a096-c19d-414d-b9c6-d1c78b3201c9",
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
      "timestamp": "2026-06-13T14:10:48.459702848Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "reference_noise",
      "diff_count": 0,
      "id": "49b1ed46-3878-4ff5-8adf-213f94607d2f",
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
      "timestamp": "2026-06-13T14:10:48.451872489Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_difference",
      "diff_count": 1,
      "id": "4a32dbb2-d91b-4853-a6ba-2db354c46f59",
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
      "timestamp": "2026-06-13T14:10:48.439620393Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "match",
      "diff_count": 0,
      "id": "5a4463d7-8058-4f11-ad2e-4f35e3b4a8a3",
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
      "timestamp": "2026-06-13T14:10:48.427453367Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "reference_noise",
      "diff_count": 0,
      "id": "5ee56edc-6183-460f-a845-95297f467bbe",
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
      "timestamp": "2026-06-13T14:10:48.427256516Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "match",
      "diff_count": 0,
      "id": "dee0f81e-073d-45ce-a7cf-98c22796e618",
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
      "timestamp": "2026-06-13T14:10:48.411598704Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_difference",
      "diff_count": 1,
      "id": "737fc430-c578-4d7b-b6ec-f845d84e9c40",
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
      "timestamp": "2026-06-13T14:10:48.398898726Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 400,
      "classification": "suspicious_difference",
      "diff_count": 3,
      "id": "67c82e3d-9fcf-4734-a99c-cf2df6749620",
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
      "timestamp": "2026-06-13T14:10:48.391379181Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "eeaa9188-078c-40ba-aa7b-2724fa11d341",
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
      "timestamp": "2026-06-13T14:10:48.380992083Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_difference",
      "diff_count": 1,
      "id": "512ec3cb-a96c-474c-98b3-081181c2504b",
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
      "timestamp": "2026-06-13T14:10:48.375160835Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "reference_noise",
      "diff_count": 0,
      "id": "9578fa45-3003-4aff-a236-e91c5f372798",
      "input": {
        "method": "GET",
        "path": "/noise",
        "query": null
      },
      "noise_count": 2,
      "primary_latency_ms": 0,
      "primary_status": 200,
      "secondary_latency_ms": 1,
      "secondary_status": 200,
      "timestamp": "2026-06-13T14:10:48.367056991Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "match",
      "diff_count": 0,
      "id": "d04aea63-98a6-495c-ace8-320da98d04cf",
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
      "timestamp": "2026-06-13T14:10:48.360228387Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "match",
      "diff_count": 0,
      "id": "f2a7b910-057c-4b38-8999-93d96e7c003d",
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
      "timestamp": "2026-06-13T14:10:48.353679007Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 400,
      "classification": "suspicious_difference",
      "diff_count": 3,
      "id": "9eab02b9-1c06-48e2-98b9-18f6abfffedf",
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
      "timestamp": "2026-06-13T14:10:48.347807393Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "reference_noise",
      "diff_count": 0,
      "id": "1f2dbe86-b2fc-421b-9afe-a0790cdc6853",
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
      "timestamp": "2026-06-13T14:10:48.347024069Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_difference",
      "diff_count": 1,
      "id": "acf90cbe-cf9a-404b-89e0-01164afd01d7",
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
      "timestamp": "2026-06-13T14:10:47.982809463Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "55be5f36-2669-4b25-9865-3ea69b416597",
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
      "timestamp": "2026-06-13T14:10:47.982002495Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 400,
      "classification": "suspicious_difference",
      "diff_count": 3,
      "id": "00e04f1c-e2d7-48ed-ae93-3faa98b1974a",
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
      "timestamp": "2026-06-13T14:10:47.970765799Z"
    },
    {
      "adapter": "http",
      "candidate_latency_ms": 0,
      "candidate_status": 200,
      "classification": "suspicious_with_noise",
      "diff_count": 1,
      "id": "8baa82b4-605b-488d-be32-56f60fd3ac40",
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
      "timestamp": "2026-06-13T14:10:47.962237608Z"
    }
  ],
  "matches": 447,
  "reference_noise": 209,
  "suspicious_differences": 424,
  "suspicious_with_noise": 210,
  "target_errors": 0,
  "total_runs": 1290
}
```
