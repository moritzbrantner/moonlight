import { cliBenchmark, type CliToolComparison, type HttpBenchmarkTarget, httpBenchmark } from "../benchmarkData";
import { demoConfig, demoRunList, demoRuns, demoStats } from "../demoData";
import type { TargetObservation } from "../types";

export const configFixture = demoConfig;
export const runFixture = demoRuns[0];
export const noiseRunFixture = demoRuns[1];
export const cliRunFixture = demoRuns[2];
export const runListFixture = demoRunList;
export const statsFixture = demoStats;
export const httpBenchmarkTargetFixture: HttpBenchmarkTarget = httpBenchmark.targets.moonlight;
export const cliBenchmarkComparisonFixture: CliToolComparison = cliBenchmark.comparisons.moonlight;

export const targetErrorFixture: TargetObservation = {
  ...runFixture.candidate,
  status: null,
  error: "connection refused",
  body: {
    size_bytes: 0,
    sha256: "empty",
    preview: "",
    truncated: false
  }
};

export const targetWithStderrFixture: TargetObservation = {
  ...runFixture.candidate,
  stderr: {
    size_bytes: 13,
    sha256: "stderr",
    preview: "warning text",
    truncated: false
  }
};

export const skippedCliComparisonFixture: CliToolComparison = {
  ...cliBenchmarkComparisonFixture,
  status: "skipped",
  total_invocations: 0,
  cases_per_invocation: 0,
  total_cases: 0,
  latency_ms: { min: null, mean: null, p50: null, p90: null, p95: null, p99: null, max: null },
  case_latency_ms: { min: null, mean: null, p50: null, p90: null, p95: null, p99: null, max: null },
  target_invocation_latency_ms: undefined,
  version: null,
  reason: "tool unavailable"
};
