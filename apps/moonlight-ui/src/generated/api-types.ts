// Generated from Moonlight Rust API models. Keep UI-only view types outside this file.

export type Classification = "match" | "suspicious_difference" | "reference_noise" | "suspicious_with_noise" | "target_error";

export type DiffKind = "status" | "header" | "body" | "stderr" | "target_error";

export type Adapter = "http" | "cli" | "project";

export type ReviewStatus = "new" | "accepted" | "ignored" | "fixed";

export type ReturnTarget = "primary" | "candidate";

export type ReturnFallback = "none" | "primary";

export type ResponseTiming = "wait_all" | "return_selected";

export type BodyCapture = { size_bytes: number, sha256: string, preview: string, truncated: boolean, };

export type TargetObservation = { status: number | null, headers: Record<string, string>, body: BodyCapture, stderr: BodyCapture | null, latency_ms: number, error: string | null, };

export type RunInput = { method: string, path: string, query: string | null, } | { eval_id: string, project: string, check_id: string, check_name: string | null, repo: string, baseline_ref: string, candidate_source: string, primary_command: string, candidate_command: string, secondary_command: string | null, } | { primary_command: string, candidate_command: string, secondary_command: string | null, };

export type DiffEntry = { kind: DiffKind, path: string, primary: string | null, candidate: string | null, secondary: string | null, message: string, };

export type ComparisonSummary = { classification: Classification, raw_candidate_diffs: Array<DiffEntry>, reference_noise: Array<DiffEntry>, noise_filtered_diffs: Array<DiffEntry>, raw_diff_summary: string, noise_summary: string, };

export type ComparisonRunListItem = { id: string, timestamp: string, adapter: Adapter, input: RunInput, primary_status: number | null, candidate_status: number | null, secondary_status: number | null, classification: Classification, primary_latency_ms: number, candidate_latency_ms: number, secondary_latency_ms: number | null, diff_count: number, noise_count: number, };

export type ComparisonRun = { id: string, timestamp: string, adapter: Adapter, input: RunInput, request_headers: Record<string, string>, request_body: BodyCapture, primary: TargetObservation, candidate: TargetObservation, secondary: TargetObservation | null, comparison: ComparisonSummary, };

export type RunPage = { items: Array<ComparisonRunListItem>, limit: number, offset: number, total: number, next_offset: number | null, };

export type RunReviewState = { run_id: string, status: ReviewStatus, note: string | null, tags: Array<string>, updated_at: string, };

export type ReviewUpdate = { status: ReviewStatus, note?: string | null, tags?: Array<string> | null, };

export type LatencyStats = { primary_avg_ms: number, candidate_avg_ms: number, secondary_avg_ms: number | null, };

export type StatsSummary = { total_runs: number, matches: number, suspicious_differences: number, reference_noise: number, suspicious_with_noise: number, target_errors: number, latency: LatencyStats, latest_runs: Array<ComparisonRunListItem>, };

export type AppConfig = { bind_addr: string, primary_url: string, candidate_url: string, secondary_url: string, enable_secondary: boolean, return_target: ReturnTarget, return_fallback: ReturnFallback, response_timing: ResponseTiming, max_body_capture_bytes: number, max_request_body_bytes: number, redact_headers: Array<string>, redact_json_paths: Array<string>, redact_json_path_patterns: Array<string>, redact_query_params: Array<string>, ignore_json_paths: Array<string>, ignore_json_path_patterns: Array<string>, ignore_headers: Array<string>, ignore_stderr: boolean, target_timeout_ms: number, storage_path: string, review_state_path: string, cors_origins: Array<string>, retention_max_runs: number | null, retention_max_bytes: number | null, };

export type MetricsClassificationCounts = { matches: number, suspicious_differences: number, reference_noise: number, suspicious_with_noise: number, target_errors: number, };

export type MetricsSnapshot = { total_proxied_comparisons_started: number, persisted_comparisons: number, persistence_failures: number, storage_refresh_failures: number, target_errors_observed: number, classifications: MetricsClassificationCounts, };
