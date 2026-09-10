import assert from "node:assert/strict";
import test from "node:test";
import {
  CLI_BENCHMARK_HISTORY_SCHEMA,
  appendHistoryDocument,
  snapshotFromReport,
} from "./append-cli-benchmark-history.mjs";

function report(shellP50, shellP95, argvP50, argvP95, gitSha = "abc123") {
  return {
    generated_at: "2026-09-10T12:00:00Z",
    config: {
      comparison_runs: 20,
      comparison_cases: 25,
      warmup: 20,
    },
    environment: {
      rustc: "rustc 1.96.0",
      cargo: "cargo 1.96.0",
      git_sha: gitSha,
    },
    comparisons: {
      moonlight: {
        status: "ok",
        total_cases: 500,
        total_target_invocations: 1000,
        latency_ms: { p50: shellP50 * 25, p95: shellP95 * 25 },
        case_latency_ms: { p50: shellP50, p95: shellP95 },
        target_invocation_latency_ms: { p50: shellP50 / 2, p95: shellP95 / 2 },
      },
      "moonlight-argv": {
        status: "ok",
        total_cases: 500,
        total_target_invocations: 1000,
        latency_ms: { p50: argvP50 * 25, p95: argvP95 * 25 },
        case_latency_ms: { p50: argvP50, p95: argvP95 },
        target_invocation_latency_ms: { p50: argvP50 / 2, p95: argvP95 / 2 },
      },
    },
  };
}

test("captures shell and direct argv performance with the relative reduction", () => {
  const snapshot = snapshotFromReport(report(4, 5, 1, 1.25), {
    commit: "commit-b",
    timestamp: "2026-09-10T12:00:00Z",
  });

  assert.equal(snapshot.shell.case_p95_ms, 5);
  assert.equal(snapshot.argv.case_p95_ms, 1.25);
  assert.equal(snapshot.argv_vs_shell.case_p50_reduction_percent, 75);
  assert.equal(snapshot.argv_vs_shell.case_p95_reduction_percent, 75);
});

test("keeps one snapshot per commit and replaces a rerun idempotently", () => {
  const first = snapshotFromReport(report(4, 5, 1, 1.25), {
    commit: "commit-a",
    timestamp: "2026-09-09T12:00:00Z",
  });
  const second = snapshotFromReport(report(4, 5, 0.8, 1), {
    commit: "commit-a",
    timestamp: "2026-09-09T12:00:00Z",
  });

  const initial = appendHistoryDocument(null, first, "moritzbrantner/moonlight", "2026-09-09T12:01:00Z");
  const rerun = appendHistoryDocument(initial, second, "moritzbrantner/moonlight", "2026-09-09T12:02:00Z");

  assert.equal(rerun.schema_version, CLI_BENCHMARK_HISTORY_SCHEMA);
  assert.equal(rerun.entries.length, 1);
  assert.equal(rerun.entries[0].argv.case_p95_ms, 1);
});

test("orders snapshots by commit timestamp", () => {
  const newer = snapshotFromReport(report(4, 5, 1, 1.25), {
    commit: "newer",
    timestamp: "2026-09-10T12:00:00Z",
  });
  const older = snapshotFromReport(report(5, 6, 2, 2.5), {
    commit: "older",
    timestamp: "2026-06-15T23:40:32Z",
  });

  const withNewer = appendHistoryDocument(null, newer, "moritzbrantner/moonlight");
  const complete = appendHistoryDocument(withNewer, older, "moritzbrantner/moonlight");

  assert.deepEqual(
    complete.entries.map((entry) => entry.commit),
    ["older", "newer"],
  );
});
