export type Classification =
  | "match"
  | "candidate_diff"
  | "noise"
  | "candidate_diff_with_noise"
  | "backend_error";

export type DiffKind = "status" | "header" | "body" | "backend_error";

export interface BodyCapture {
  size_bytes: number;
  sha256: string;
  preview: string;
  truncated: boolean;
}

export interface BackendCapture {
  status: number | null;
  headers: Record<string, string>;
  body: BodyCapture;
  latency_ms: number;
  error: string | null;
}

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

export interface RequestListItem {
  id: string;
  timestamp: string;
  method: string;
  path: string;
  query: string | null;
  primary_status: number | null;
  candidate_status: number | null;
  secondary_status: number | null;
  classification: Classification;
  primary_latency_ms: number;
  candidate_latency_ms: number | null;
  secondary_latency_ms: number | null;
  diff_count: number;
  noise_count: number;
}

export interface RequestRecord {
  id: string;
  timestamp: string;
  method: string;
  path: string;
  query: string | null;
  request_headers: Record<string, string>;
  request_body: BodyCapture;
  primary: BackendCapture;
  candidate: BackendCapture | null;
  secondary: BackendCapture | null;
  comparison: ComparisonSummary;
}

export interface StatsSummary {
  total_requests: number;
  matches: number;
  candidate_diffs: number;
  noise: number;
  candidate_diff_with_noise: number;
  backend_errors: number;
  latency: {
    primary_avg_ms: number;
    candidate_avg_ms: number | null;
    secondary_avg_ms: number | null;
  };
  latest_requests: RequestListItem[];
}

export interface AppConfig {
  bind_addr: string;
  primary_url: string;
  candidate_url: string;
  secondary_url: string;
  enable_candidate: boolean;
  enable_secondary: boolean;
  return_backend: "primary";
  max_body_capture_bytes: number;
  redact_headers: string[];
  ignored_json_paths: string[];
  ignored_headers: string[];
  storage_path: string;
}
