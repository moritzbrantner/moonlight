import cliBenchmarkReport from "../../../data/moonlight/cli-benchmark-analysis/latest.json";
import httpBenchmarkReport from "../../../data/moonlight/benchmark/latest.json";

export type PercentileSummary = {
  min: number | null;
  mean: number | null;
  p50: number | null;
  p90: number | null;
  p95: number | null;
  p99: number | null;
  max: number | null;
};

export type HttpBenchmarkTarget = {
  name: string;
  total_requests: number;
  success_count: number;
  error_count: number;
  requests_per_second: number;
  latency_ms: PercentileSummary;
  status_counts: Record<string, number>;
};

export type HttpBenchmarkReport = {
  generated_at: string;
  config: {
    concurrency: number;
    endpoints: string[];
    requests: number;
    validation_requests: number;
    warmup: number;
  };
  targets: Record<string, HttpBenchmarkTarget>;
  validity: Array<{
    endpoint: string;
    match: boolean;
    mismatches: string[];
  }>;
};

export type CliToolComparison = {
  status: string;
  total_invocations: number;
  cases_per_invocation: number;
  total_cases: number;
  target_invocations_per_case?: number;
  total_target_invocations?: number;
  latency_ms: PercentileSummary;
  case_latency_ms: PercentileSummary;
  target_invocation_latency_ms?: PercentileSummary;
  version: string | null;
  reason: string | null;
};

export type CliScenarioBenchmark = {
  classifications: Record<string, number>;
  error_count: number;
  latency_ms: PercentileSummary;
  records_written: number;
  success_count: number;
  total_invocations: number;
  validation_errors: string[];
};

export type CliBenchmarkReport = {
  generated_at: string;
  config: {
    comparison_cases: number;
    comparison_runs: number;
    concurrency: number;
    requests: number;
    scenarios: string[];
    targets: string[];
    warmup: number;
  };
  comparisons: Record<string, CliToolComparison>;
  environment: {
    cargo: string;
    git_sha: string;
    rustc: string;
  };
  scenarios: Record<string, CliScenarioBenchmark>;
};

export const httpBenchmark = httpBenchmarkReport as HttpBenchmarkReport;
export const cliBenchmark = cliBenchmarkReport as CliBenchmarkReport;
