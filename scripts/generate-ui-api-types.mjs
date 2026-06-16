import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const outputPath = resolve("apps/moonlight-ui/src/generated/api-types.ts");

const content = `// Generated from Moonlight Rust API models. Keep UI-only view types outside this file.
export type Classification =
  | "match"
  | "suspicious_difference"
  | "reference_noise"
  | "suspicious_with_noise"
  | "target_error";

export type DiffKind = "status" | "header" | "body" | "stderr" | "target_error";

export type Adapter = "http" | "cli";

export type ReviewStatus = "new" | "accepted" | "ignored" | "fixed";

export interface BodyCapture {
  size_bytes: number;
  sha256: string;
  preview: string;
  truncated: boolean;
}

export interface TargetObservation {
  status: number | null;
  headers: Record<string, string>;
  body: BodyCapture;
  stderr: BodyCapture | null;
  latency_ms: number;
  error: string | null;
}

export interface HttpRunInput {
  method: string;
  path: string;
  query: string | null;
}

export interface CliRunInput {
  primary_command: string;
  candidate_command: string;
  secondary_command: string | null;
}

export type RunInput = HttpRunInput | CliRunInput;

export interface DiffEntry {
  kind: DiffKind;
  path: string;
  primary: string | null;
  candidate: string | null;
  secondary: string | null;
  message: string;
}

export interface ComparisonSummary {
  classification: Classification;
  raw_candidate_diffs: DiffEntry[];
  reference_noise: DiffEntry[];
  noise_filtered_diffs: DiffEntry[];
  raw_diff_summary: string;
  noise_summary: string;
}

export interface ComparisonRunListItem {
  id: string;
  timestamp: string;
  adapter: Adapter;
  input: RunInput;
  primary_status: number | null;
  candidate_status: number | null;
  secondary_status: number | null;
  classification: Classification;
  primary_latency_ms: number;
  candidate_latency_ms: number;
  secondary_latency_ms: number | null;
  diff_count: number;
  noise_count: number;
}

export interface ComparisonRun {
  id: string;
  timestamp: string;
  adapter: Adapter;
  input: RunInput;
  request_headers: Record<string, string>;
  request_body: BodyCapture;
  primary: TargetObservation;
  candidate: TargetObservation;
  secondary: TargetObservation | null;
  comparison: ComparisonSummary;
}

export interface RunPage {
  items: ComparisonRunListItem[];
  limit: number;
  offset: number;
  total: number;
  next_offset: number | null;
}

export interface RunReviewState {
  run_id: string;
  status: ReviewStatus;
  note: string | null;
  tags: string[];
  updated_at: string;
}

export interface ReviewUpdate {
  status: ReviewStatus;
  note?: string | null;
  tags?: string[];
}

export interface StatsSummary {
  total_runs: number;
  matches: number;
  suspicious_differences: number;
  reference_noise: number;
  suspicious_with_noise: number;
  target_errors: number;
  latency: {
    primary_avg_ms: number;
    candidate_avg_ms: number;
    secondary_avg_ms: number | null;
  };
  latest_runs: ComparisonRunListItem[];
}

export interface AppConfig {
  bind_addr: string;
  primary_url: string;
  candidate_url: string;
  secondary_url: string;
  enable_secondary: boolean;
  return_target: "primary" | "candidate";
  return_fallback: "none" | "primary";
  response_timing: "wait_all" | "return_selected";
  max_body_capture_bytes: number;
  max_request_body_bytes: number;
  redact_headers: string[];
  redact_json_paths: string[];
  redact_json_path_patterns: string[];
  redact_query_params: string[];
  ignore_json_paths: string[];
  ignore_json_path_patterns: string[];
  ignore_headers: string[];
  ignore_stderr: boolean;
  target_timeout_ms: number;
  storage_path: string;
  review_state_path: string;
  cors_origins: string[];
  retention_max_runs: number | null;
  retention_max_bytes: number | null;
}
`;

if (process.argv.includes("--check")) {
  const current = readFileSync(outputPath, "utf8");
  if (current !== content) {
    console.error(`${outputPath} is stale. Run: bun scripts/generate-ui-api-types.mjs`);
    process.exit(1);
  }
} else {
  writeFileSync(outputPath, content);
}
