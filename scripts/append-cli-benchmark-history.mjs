import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const CLI_BENCHMARK_HISTORY_SCHEMA = "moonlight/cli-benchmark-history/v1";
export const CLI_BENCHMARK_HISTORY_RETENTION = 1000;

function finiteOrNull(value) {
  return Number.isFinite(value) ? value : null;
}

function percentileSnapshot(comparison) {
  if (!comparison || typeof comparison !== "object") {
    return null;
  }

  return {
    status: typeof comparison.status === "string" ? comparison.status : "unknown",
    suite_p50_ms: finiteOrNull(comparison.latency_ms?.p50),
    suite_p95_ms: finiteOrNull(comparison.latency_ms?.p95),
    case_p50_ms: finiteOrNull(comparison.case_latency_ms?.p50),
    case_p95_ms: finiteOrNull(comparison.case_latency_ms?.p95),
    target_p50_ms: finiteOrNull(comparison.target_invocation_latency_ms?.p50),
    target_p95_ms: finiteOrNull(comparison.target_invocation_latency_ms?.p95),
    total_cases: Number.isInteger(comparison.total_cases) ? comparison.total_cases : null,
    total_target_invocations: Number.isInteger(comparison.total_target_invocations)
      ? comparison.total_target_invocations
      : null,
  };
}

function reductionPercent(shellValue, argvValue) {
  if (!Number.isFinite(shellValue) || !Number.isFinite(argvValue) || shellValue <= 0) {
    return null;
  }
  return ((shellValue - argvValue) / shellValue) * 100;
}

export function snapshotFromReport(report, { commit, timestamp }) {
  if (!report || typeof report !== "object") {
    throw new Error("Benchmark report must be a JSON object.");
  }
  if (typeof commit !== "string" || commit.trim() === "") {
    throw new Error("A non-empty commit SHA is required.");
  }
  if (typeof timestamp !== "string" || Number.isNaN(Date.parse(timestamp))) {
    throw new Error("A valid commit timestamp is required.");
  }

  const shell = percentileSnapshot(report.comparisons?.moonlight);
  const argv = percentileSnapshot(report.comparisons?.["moonlight-argv"]);

  return {
    commit,
    timestamp,
    generated_at: typeof report.generated_at === "string" ? report.generated_at : null,
    environment: {
      rustc: typeof report.environment?.rustc === "string" ? report.environment.rustc : null,
      cargo: typeof report.environment?.cargo === "string" ? report.environment.cargo : null,
      benchmark_git_sha:
        typeof report.environment?.git_sha === "string" ? report.environment.git_sha : null,
    },
    config: {
      comparison_runs: Number.isInteger(report.config?.comparison_runs)
        ? report.config.comparison_runs
        : null,
      comparison_cases: Number.isInteger(report.config?.comparison_cases)
        ? report.config.comparison_cases
        : null,
      warmup: Number.isInteger(report.config?.warmup) ? report.config.warmup : null,
    },
    shell,
    argv,
    argv_vs_shell: {
      case_p50_reduction_percent: reductionPercent(shell?.case_p50_ms, argv?.case_p50_ms),
      case_p95_reduction_percent: reductionPercent(shell?.case_p95_ms, argv?.case_p95_ms),
      target_p50_reduction_percent: reductionPercent(shell?.target_p50_ms, argv?.target_p50_ms),
      target_p95_reduction_percent: reductionPercent(shell?.target_p95_ms, argv?.target_p95_ms),
    },
  };
}

export function appendHistoryDocument(document, snapshot, repository, updatedAt = new Date().toISOString()) {
  if (typeof repository !== "string" || repository.trim() === "") {
    throw new Error("A non-empty repository is required.");
  }

  if (document != null) {
    if (document.schema_version !== CLI_BENCHMARK_HISTORY_SCHEMA) {
      throw new Error(`Expected ${CLI_BENCHMARK_HISTORY_SCHEMA}.`);
    }
    if (document.repository !== repository) {
      throw new Error(`History belongs to ${document.repository}, not ${repository}.`);
    }
  }

  const entries = Array.isArray(document?.entries)
    ? document.entries.filter((entry) => entry?.commit !== snapshot.commit)
    : [];
  entries.push(snapshot);
  entries.sort((left, right) => {
    const timestampOrder = Date.parse(left.timestamp) - Date.parse(right.timestamp);
    return timestampOrder === 0 ? left.commit.localeCompare(right.commit) : timestampOrder;
  });

  return {
    schema_version: CLI_BENCHMARK_HISTORY_SCHEMA,
    repository,
    updated_at: updatedAt,
    entries: entries.slice(-CLI_BENCHMARK_HISTORY_RETENTION),
  };
}

function parseArguments(argv) {
  const options = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || value == null) {
      throw new Error(`Invalid argument sequence near ${key ?? "<end>"}.`);
    }
    options.set(key.slice(2), value);
  }
  return options;
}

async function readHistory(path) {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch (error) {
    if (error?.code === "ENOENT") {
      return null;
    }
    throw error;
  }
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  const historyPath = options.get("history");
  const reportPath = options.get("report");
  const repository = options.get("repository");
  const commit = options.get("commit");
  const timestamp = options.get("timestamp");

  for (const [name, value] of [
    ["history", historyPath],
    ["report", reportPath],
    ["repository", repository],
    ["commit", commit],
    ["timestamp", timestamp],
  ]) {
    if (!value) {
      throw new Error(`Missing --${name}.`);
    }
  }

  const report = JSON.parse(await readFile(reportPath, "utf8"));
  const current = await readHistory(historyPath);
  const snapshot = snapshotFromReport(report, { commit, timestamp });
  const next = appendHistoryDocument(current, snapshot, repository);

  await mkdir(dirname(historyPath), { recursive: true });
  await writeFile(historyPath, `${JSON.stringify(next, null, 2)}\n`);
}

const isDirectExecution = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isDirectExecution) {
  await main();
}
