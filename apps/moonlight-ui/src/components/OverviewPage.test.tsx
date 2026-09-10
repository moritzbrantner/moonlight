import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  CLI_BENCHMARK_HISTORY_SCHEMA,
  CLI_BENCHMARK_HISTORY_URL,
  CLI_BENCHMARK_PUBLISHED_LATEST_URL,
  cliBenchmark,
  httpBenchmark,
} from "../benchmarkData";
import { OverviewPage } from "./OverviewPage";

describe("OverviewPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("published evidence unavailable in unit test")));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders hero, repository content, and benchmark summary", () => {
    render(<OverviewPage onNavigate={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "Reference and candidate checks for HTTP and CLI targets." })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /repository/i })).toHaveAttribute("href", "https://github.com/moritzbrantner/moonlight");
    expect(screen.getByLabelText("Latest benchmark summary")).toBeInTheDocument();
    expect(screen.getByText("moonlight-core")).toBeInTheDocument();
  });

  it("renders HTTP and CLI benchmark rows and promotes direct argv", () => {
    render(<OverviewPage onNavigate={vi.fn()} />);

    for (const target of ["moonlight", "diffy_b", "diffy_c"]) {
      expect(screen.getAllByText(httpBenchmark.targets[target].name).length).toBeGreaterThan(0);
    }

    for (const target of cliBenchmark.config.targets) {
      expect(screen.getAllByText(target).length).toBeGreaterThan(0);
    }

    expect(screen.getByText(/CLI argv p95 ms\/case/i)).toBeInTheDocument();
    expect(screen.getByLabelText("Direct argv performance summary")).toHaveTextContent(
      /Direct argv is the preferred fast path/i,
    );
  });

  it("loads the published exact-head benchmark and commit history", async () => {
    const published = {
      ...cliBenchmark,
      generated_at: "2026-09-10T12:00:00Z",
      environment: { ...cliBenchmark.environment, git_sha: "current123456789" },
    };
    const history = {
      schema_version: CLI_BENCHMARK_HISTORY_SCHEMA,
      repository: "moritzbrantner/moonlight",
      updated_at: "2026-09-10T12:05:00Z",
      entries: [
        {
          commit: "older123456789",
          timestamp: "2026-06-15T23:40:32Z",
          generated_at: "2026-06-15T23:40:32Z",
          environment: { rustc: "rustc 1.96.0", cargo: "cargo 1.96.0", benchmark_git_sha: "older123456789" },
          config: { comparison_runs: 20, comparison_cases: 25, warmup: 20 },
          shell: { status: "ok", suite_p50_ms: 111, suite_p95_ms: 124, case_p50_ms: 4.45, case_p95_ms: 4.96, target_p50_ms: 2.22, target_p95_ms: 2.48, total_cases: 500, total_target_invocations: 1000 },
          argv: { status: "ok", suite_p50_ms: 26, suite_p95_ms: 32, case_p50_ms: 1.05, case_p95_ms: 1.29, target_p50_ms: 0.52, target_p95_ms: 0.64, total_cases: 500, total_target_invocations: 1000 },
          argv_vs_shell: { case_p50_reduction_percent: 76.4, case_p95_reduction_percent: 74, target_p50_reduction_percent: 76.6, target_p95_reduction_percent: 74.2 },
        },
        {
          commit: "current123456789",
          timestamp: "2026-09-10T12:00:00Z",
          generated_at: "2026-09-10T12:00:00Z",
          environment: { rustc: "rustc 1.96.0", cargo: "cargo 1.96.0", benchmark_git_sha: "current123456789" },
          config: { comparison_runs: 20, comparison_cases: 25, warmup: 20 },
          shell: { status: "ok", suite_p50_ms: 100, suite_p95_ms: 110, case_p50_ms: 4, case_p95_ms: 4.4, target_p50_ms: 2, target_p95_ms: 2.2, total_cases: 500, total_target_invocations: 1000 },
          argv: { status: "ok", suite_p50_ms: 20, suite_p95_ms: 25, case_p50_ms: 0.8, case_p95_ms: 1, target_p50_ms: 0.4, target_p95_ms: 0.5, total_cases: 500, total_target_invocations: 1000 },
          argv_vs_shell: { case_p50_reduction_percent: 80, case_p95_reduction_percent: 77.3, target_p50_reduction_percent: 80, target_p95_reduction_percent: 77.3 },
        },
      ],
    };

    vi.stubGlobal(
      "fetch",
      vi.fn((input: string | URL | Request) => {
        const url = String(input);
        const body = url === CLI_BENCHMARK_PUBLISHED_LATEST_URL ? published : history;
        expect([CLI_BENCHMARK_PUBLISHED_LATEST_URL, CLI_BENCHMARK_HISTORY_URL]).toContain(url);
        return Promise.resolve({ ok: true, json: () => Promise.resolve(body) });
      }),
    );

    render(<OverviewPage onNavigate={vi.fn()} />);

    expect(await screen.findByRole("img", { name: /CLI p95 latency history/i })).toBeInTheDocument();
    expect(await screen.findByText(/published exact-head evidence/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /latest current1/i })).toHaveAttribute(
      "href",
      "https://github.com/moritzbrantner/moonlight/commit/current123456789",
    );
  });

  it("navigates to dashboard from hero action", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();
    render(<OverviewPage onNavigate={onNavigate} />);

    await user.click(screen.getByRole("button", { name: /demo dashboard/i }));
    expect(onNavigate).toHaveBeenCalledWith("dashboard");
  });
});
