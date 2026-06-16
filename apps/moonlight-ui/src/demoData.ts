import type { AppConfig, ComparisonRun, ComparisonRunListItem, RunReviewState, StatsSummary, TargetObservation } from "./types";

const now = new Date("2026-06-13T12:00:00.000Z");

function body(preview: string) {
  return {
    size_bytes: preview.length,
    sha256: "demo",
    preview,
    truncated: false
  };
}

function observation(status: number, preview: string, latency_ms: number): TargetObservation {
  return {
    status,
    headers: {
      "content-type": "application/json"
    },
    body: body(preview),
    stderr: null,
    latency_ms,
    error: null
  };
}

export const demoConfig: AppConfig = {
  bind_addr: "127.0.0.1:8080",
  primary_url: "https://primary.example.test",
  candidate_url: "https://candidate.example.test",
  secondary_url: "https://secondary.example.test",
  enable_secondary: true,
  return_target: "primary",
  return_fallback: "none",
  response_timing: "wait_all",
  max_body_capture_bytes: 8192,
  max_request_body_bytes: 10485760,
  redact_headers: [
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "proxy-authorization",
    "x-auth-token",
    "x-csrf-token"
  ],
  redact_json_paths: [],
  redact_json_path_patterns: ["$.items[*].secret"],
  redact_query_params: ["token", "access_token", "id_token", "api_key", "key", "secret", "password"],
  ignore_json_paths: ["$.generated_at"],
  ignore_json_path_patterns: ["$.items[*].id"],
  ignore_headers: ["date"],
  ignore_stderr: false,
  target_timeout_ms: 30000,
  storage_path: "data/moonlight/http-runs.jsonl",
  review_state_path: "data/moonlight/review-state.json",
  cors_origins: ["http://127.0.0.1:5173", "http://localhost:5173"],
  retention_max_runs: null,
  retention_max_bytes: null
};

export const demoRuns: ComparisonRun[] = [
  {
    id: "demo-regression",
    timestamp: now.toISOString(),
    adapter: "http",
    input: {
      method: "GET",
      path: "/regression",
      query: null
    },
    request_headers: {
      accept: "application/json"
    },
    request_body: body(""),
    primary: observation(200, "{\"status\":\"ok\",\"value\":42}", 18),
    candidate: observation(200, "{\"status\":\"ok\",\"value\":43}", 24),
    secondary: observation(200, "{\"status\":\"ok\",\"value\":42}", 21),
    comparison: {
      classification: "suspicious_difference",
      raw_candidate_diffs: [
        {
          kind: "body",
          path: "$.value",
          primary: "42",
          candidate: "43",
          secondary: "42",
          message: "Candidate value differs from both references."
        }
      ],
      reference_noise: [],
      noise_filtered_diffs: [
        {
          kind: "body",
          path: "$.value",
          primary: "42",
          candidate: "43",
          secondary: "42",
          message: "Candidate value differs from both references."
        }
      ],
      raw_diff_summary: "1 candidate diff",
      noise_summary: "No reference noise"
    }
  },
  {
    id: "demo-noise",
    timestamp: new Date(now.getTime() - 42_000).toISOString(),
    adapter: "http",
    input: {
      method: "GET",
      path: "/noise",
      query: "demo=true"
    },
    request_headers: {
      accept: "application/json"
    },
    request_body: body(""),
    primary: observation(200, "{\"status\":\"ok\",\"generated_at\":\"12:00:00\"}", 19),
    candidate: observation(200, "{\"status\":\"ok\",\"generated_at\":\"12:00:01\"}", 20),
    secondary: observation(200, "{\"status\":\"ok\",\"generated_at\":\"12:00:02\"}", 23),
    comparison: {
      classification: "reference_noise",
      raw_candidate_diffs: [
        {
          kind: "body",
          path: "$.generated_at",
          primary: "12:00:00",
          candidate: "12:00:01",
          secondary: "12:00:02",
          message: "Timestamp differs across references."
        }
      ],
      reference_noise: [
        {
          kind: "body",
          path: "$.generated_at",
          primary: "12:00:00",
          candidate: "12:00:01",
          secondary: "12:00:02",
          message: "Reference values are unstable."
        }
      ],
      noise_filtered_diffs: [],
      raw_diff_summary: "1 candidate diff",
      noise_summary: "1 noisy field filtered"
    }
  },
  {
    id: "demo-cli",
    timestamp: new Date(now.getTime() - 86_000).toISOString(),
    adapter: "cli",
    input: {
      primary_command: "printf '{\"value\":42}\\n'",
      candidate_command: "printf '{\"value\":42}\\n'",
      secondary_command: "printf '{\"value\":42}\\n'"
    },
    request_headers: {},
    request_body: body(""),
    primary: observation(0, "{\"value\":42}", 8),
    candidate: observation(0, "{\"value\":42}", 9),
    secondary: observation(0, "{\"value\":42}", 8),
    comparison: {
      classification: "match",
      raw_candidate_diffs: [],
      reference_noise: [],
      noise_filtered_diffs: [],
      raw_diff_summary: "No candidate differences",
      noise_summary: "No reference noise"
    }
  },
  {
    id: "demo-timeout",
    timestamp: new Date(now.getTime() - 132_000).toISOString(),
    adapter: "http",
    input: {
      method: "GET",
      path: "/slow-candidate",
      query: null
    },
    request_headers: {
      accept: "application/json"
    },
    request_body: body(""),
    primary: observation(200, "{\"status\":\"ok\"}", 17),
    candidate: {
      ...observation(0, "", 30001),
      status: null,
      error: "candidate request timed out after 30000 ms"
    },
    secondary: observation(200, "{\"status\":\"ok\"}", 19),
    comparison: {
      classification: "target_error",
      raw_candidate_diffs: [
        {
          kind: "target_error",
          path: "$target_error",
          primary: null,
          candidate: "candidate request timed out after 30000 ms",
          secondary: null,
          message: "primary target error differs from candidate"
        }
      ],
      reference_noise: [],
      noise_filtered_diffs: [
        {
          kind: "target_error",
          path: "$target_error",
          primary: null,
          candidate: "candidate request timed out after 30000 ms",
          secondary: null,
          message: "primary target error differs from candidate"
        }
      ],
      raw_diff_summary: "candidate: 1 diff(s)",
      noise_summary: "no reference noise diffs"
    }
  }
];

export const demoRunList: ComparisonRunListItem[] = demoRuns.map((run) => ({
  id: run.id,
  timestamp: run.timestamp,
  adapter: run.adapter,
  input: run.input,
  primary_status: run.primary.status,
  candidate_status: run.candidate.status,
  secondary_status: run.secondary?.status ?? null,
  classification: run.comparison.classification,
  primary_latency_ms: run.primary.latency_ms,
  candidate_latency_ms: run.candidate.latency_ms,
  secondary_latency_ms: run.secondary?.latency_ms ?? null,
  diff_count: run.comparison.noise_filtered_diffs.length,
  noise_count: run.comparison.reference_noise.length
}));

export const demoStats: StatsSummary = {
  total_runs: demoRuns.length,
  matches: 1,
  suspicious_differences: 1,
  reference_noise: 1,
  suspicious_with_noise: 0,
  target_errors: 1,
  latency: {
    primary_avg_ms: 15,
    candidate_avg_ms: 17.7,
    secondary_avg_ms: 17.3
  },
  latest_runs: demoRunList
};

export const demoReviewStates: Record<string, RunReviewState> = {
  "demo-regression": {
    run_id: "demo-regression",
    status: "accepted",
    note: "Candidate changed the returned value.",
    tags: ["regression"],
    updated_at: now.toISOString()
  },
  "demo-noise": {
    run_id: "demo-noise",
    status: "ignored",
    note: "Generated timestamp noise.",
    tags: ["noise"],
    updated_at: new Date(now.getTime() - 30_000).toISOString()
  }
};
