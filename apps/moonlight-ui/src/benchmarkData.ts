import cliBenchmarkReport from "../../../data/moonlight/cli-benchmark-analysis/latest.json";
import httpBenchmarkReport from "../../../data/moonlight/benchmark/latest.json";

export const CLI_BENCHMARK_HISTORY_SCHEMA = "moonlight/cli-benchmark-history/v1";
export const CLI_BENCHMARK_PUBLISHED_LATEST_URL =
  "https://raw.githubusercontent.com/moritzbrantner/moonlight/cli-benchmark-history/latest.json";
export const CLI_BENCHMARK_HISTORY_URL =
  "https://raw.githubusercontent.com/moritzbrantner/moonlight/cli-benchmark-history/history.json";

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

export type CliBenchmarkHistoryMetrics = {
  status: string;
  suite_p50_ms: number | null;
  suite_p95_ms: number | null;
  case_p50_ms: number | null;
  case_p95_ms: number | null;
  target_p50_ms: number | null;
  target_p95_ms: number | null;
  total_cases: number | null;
  total_target_invocations: number | null;
};

export type CliBenchmarkHistoryEntry = {
  commit: string;
  timestamp: string;
  generated_at: string | null;
  environment: {
    rustc: string | null;
    cargo: string | null;
    benchmark_git_sha: string | null;
  };
  config: {
    comparison_runs: number | null;
    comparison_cases: number | null;
    warmup: number | null;
  };
  shell: CliBenchmarkHistoryMetrics | null;
  argv: CliBenchmarkHistoryMetrics | null;
  argv_vs_shell: {
    case_p50_reduction_percent: number | null;
    case_p95_reduction_percent: number | null;
    target_p50_reduction_percent: number | null;
    target_p95_reduction_percent: number | null;
  };
};

export type CliBenchmarkHistory = {
  schema_version: typeof CLI_BENCHMARK_HISTORY_SCHEMA;
  repository: string;
  updated_at: string;
  entries: CliBenchmarkHistoryEntry[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function isCliBenchmarkReport(value: unknown): value is CliBenchmarkReport {
  if (!isRecord(value)) {
    return false;
  }
  return (
    typeof value.generated_at === "string" &&
    isRecord(value.config) &&
    isRecord(value.comparisons) &&
    isRecord(value.environment) &&
    typeof value.environment.git_sha === "string" &&
    isRecord(value.scenarios)
  );
}

export function isCliBenchmarkHistory(value: unknown): value is CliBenchmarkHistory {
  if (!isRecord(value) || value.schema_version !== CLI_BENCHMARK_HISTORY_SCHEMA) {
    return false;
  }
  return (
    typeof value.repository === "string" &&
    typeof value.updated_at === "string" &&
    Array.isArray(value.entries) &&
    value.entries.every(
      (entry) =>
        isRecord(entry) && typeof entry.commit === "string" && typeof entry.timestamp === "string",
    )
  );
}

export function preferredCliComparison(report: CliBenchmarkReport) {
  const argv = report.comparisons["moonlight-argv"];
  if (argv?.status === "ok") {
    return { name: "moonlight-argv", comparison: argv } as const;
  }
  return { name: "moonlight", comparison: report.comparisons.moonlight } as const;
}

export function argvCaseReductionPercent(report: CliBenchmarkReport, percentile: "p50" | "p95") {
  const shell = report.comparisons.moonlight?.case_latency_ms[percentile];
  const argv = report.comparisons["moonlight-argv"]?.case_latency_ms[percentile];
  if (!Number.isFinite(shell) || !Number.isFinite(argv) || shell == null || argv == null || shell <= 0) {
    return null;
  }
  return ((shell - argv) / shell) * 100;
}

export const httpBenchmark = httpBenchmarkReport as HttpBenchmarkReport;
export const cliBenchmark = cliBenchmarkReport as CliBenchmarkReport;
