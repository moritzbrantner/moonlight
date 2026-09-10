import { Activity, GitBranch } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  CLI_BENCHMARK_HISTORY_URL,
  CLI_BENCHMARK_PUBLISHED_LATEST_URL,
  argvCaseReductionPercent,
  cliBenchmark as committedCliBenchmark,
  httpBenchmark,
  isCliBenchmarkHistory,
  isCliBenchmarkReport,
  preferredCliComparison,
  type CliBenchmarkHistory,
  type CliBenchmarkHistoryEntry,
  type CliToolComparison,
} from "../benchmarkData";
import { navigate, type Page } from "../navigation";
import { formatMs, formatNumber } from "../utils/format";
import { BenchmarkSection } from "./BenchmarkSection";
import { CliBenchmarkRow } from "./CliBenchmarkRow";
import { HttpBenchmarkRow } from "./HttpBenchmarkRow";

type OverviewPageProps = {
  onNavigate: (page: Page) => void;
};

function shortCommit(commit: string) {
  return commit.slice(0, 8);
}

function benchmarkCommitUrl(commit: string) {
  return `https://github.com/moritzbrantner/moonlight/commit/${commit}`;
}

function historyChartEntries(history: CliBenchmarkHistory | null) {
  if (!history) {
    return [];
  }
  return history.entries
    .filter(
      (entry) =>
        entry.shell?.case_p95_ms != null &&
        Number.isFinite(entry.shell.case_p95_ms) &&
        entry.argv?.case_p95_ms != null &&
        Number.isFinite(entry.argv.case_p95_ms),
    )
    .slice(-60);
}

function chartPolyline(
  entries: CliBenchmarkHistoryEntry[],
  value: (entry: CliBenchmarkHistoryEntry) => number,
  maxValue: number,
) {
  const width = 720;
  const height = 220;
  const left = 42;
  const right = 16;
  const top = 18;
  const bottom = 34;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const divisor = Math.max(entries.length - 1, 1);

  return entries
    .map((entry, index) => {
      const x = left + (index / divisor) * plotWidth;
      const y = top + (1 - value(entry) / maxValue) * plotHeight;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

function CliPerformanceHistory({ history }: { history: CliBenchmarkHistory | null }) {
  const entries = useMemo(() => historyChartEntries(history), [history]);

  if (!history) {
    return (
      <div className="benchmark-history benchmark-history--pending">
        Published commit history is not available yet. The committed benchmark remains visible as a fallback.
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="benchmark-history benchmark-history--pending">
        Benchmark history exists, but it does not yet contain comparable shell and argv p95 measurements.
      </div>
    );
  }

  const values = entries.flatMap((entry) => [entry.shell!.case_p95_ms!, entry.argv!.case_p95_ms!]);
  const maxValue = Math.max(...values, 1) * 1.08;
  const shellPoints = chartPolyline(entries, (entry) => entry.shell!.case_p95_ms!, maxValue);
  const argvPoints = chartPolyline(entries, (entry) => entry.argv!.case_p95_ms!, maxValue);
  const first = entries[0];
  const latest = entries.at(-1)!;

  return (
    <figure className="benchmark-history">
      <div className="benchmark-history__heading">
        <div>
          <strong>CLI p95 latency through commits</strong>
          <span>Lower is better. Up to the latest 60 comparable main-branch snapshots are shown.</span>
        </div>
        <a href={benchmarkCommitUrl(latest.commit)}>latest {shortCommit(latest.commit)}</a>
      </div>
      <svg
        className="benchmark-history__chart"
        viewBox="0 0 720 220"
        role="img"
        aria-label="CLI p95 latency history comparing shell commands with direct argv"
      >
        <line x1="42" y1="186" x2="704" y2="186" className="benchmark-history__axis" />
        <line x1="42" y1="18" x2="42" y2="186" className="benchmark-history__axis" />
        <text x="6" y="24" className="benchmark-history__axis-label">
          {formatNumber(maxValue, 1)} ms
        </text>
        <text x="20" y="190" className="benchmark-history__axis-label">
          0
        </text>
        <polyline points={shellPoints} className="benchmark-history__line benchmark-history__line--shell" />
        <polyline points={argvPoints} className="benchmark-history__line benchmark-history__line--argv" />
        <text x="42" y="210" className="benchmark-history__commit-label">
          {shortCommit(first.commit)}
        </text>
        <text x="704" y="210" textAnchor="end" className="benchmark-history__commit-label">
          {shortCommit(latest.commit)}
        </text>
      </svg>
      <figcaption className="benchmark-history__legend">
        <span><i className="benchmark-history__key benchmark-history__key--argv" />direct argv</span>
        <span><i className="benchmark-history__key benchmark-history__key--shell" />shell command compatibility path</span>
        {entries.length === 1 ? <span>one published point so far</span> : <span>{entries.length} published commits</span>}
      </figcaption>
    </figure>
  );
}

export function OverviewPage({ onNavigate }: OverviewPageProps) {
  const [cliBenchmark, setCliBenchmark] = useState(committedCliBenchmark);
  const [cliHistory, setCliHistory] = useState<CliBenchmarkHistory | null>(null);
  const [usingPublishedCliEvidence, setUsingPublishedCliEvidence] = useState(false);

  useEffect(() => {
    let active = true;

    async function loadPublishedEvidence() {
      const [latestResult, historyResult] = await Promise.allSettled([
        fetch(CLI_BENCHMARK_PUBLISHED_LATEST_URL, { cache: "no-store" }).then(async (response) => {
          if (!response.ok) {
            throw new Error(`Published CLI benchmark returned ${response.status}.`);
          }
          return response.json() as Promise<unknown>;
        }),
        fetch(CLI_BENCHMARK_HISTORY_URL, { cache: "no-store" }).then(async (response) => {
          if (!response.ok) {
            throw new Error(`CLI benchmark history returned ${response.status}.`);
          }
          return response.json() as Promise<unknown>;
        }),
      ]);

      if (!active) {
        return;
      }
      if (latestResult.status === "fulfilled" && isCliBenchmarkReport(latestResult.value)) {
        setCliBenchmark(latestResult.value);
        setUsingPublishedCliEvidence(true);
      }
      if (historyResult.status === "fulfilled" && isCliBenchmarkHistory(historyResult.value)) {
        setCliHistory(historyResult.value);
      }
    }

    void loadPublishedEvidence();
    return () => {
      active = false;
    };
  }, []);

  const httpTargets = ["moonlight", "diffy_b", "diffy_c"]
    .map((key) => httpBenchmark.targets[key])
    .filter(Boolean);
  const configuredCliTargets = cliBenchmark.config.targets?.length
    ? cliBenchmark.config.targets
    : Object.keys(cliBenchmark.comparisons);
  const cliTools = configuredCliTargets
    .map((key) => [key, cliBenchmark.comparisons[key]] as const)
    .filter((entry): entry is readonly [string, CliToolComparison] => Boolean(entry[1]));
  const preferredCli = preferredCliComparison(cliBenchmark);
  const argvP95Reduction = argvCaseReductionPercent(cliBenchmark, "p95");
  const shellP95 = cliBenchmark.comparisons.moonlight?.case_latency_ms.p95 ?? null;
  const argvP95 = cliBenchmark.comparisons["moonlight-argv"]?.case_latency_ms.p95 ?? null;

  return (
    <section className="overview-page">
      <section className="overview-hero">
        <div className="hero__content">
          <p className="eyebrow">Behavior comparison</p>
          <h1>Reference and candidate checks for HTTP and CLI targets.</h1>
          <p className="hero__lede">
            Moonlight fans out an input to a primary reference, a candidate, and optionally a secondary reference. It stores target observations, filters known reference noise, and classifies the remaining candidate behavior.
          </p>
          <div className="hero__actions" aria-label="Repository resources">
            <a className="button button--primary" href="https://github.com/moritzbrantner/moonlight">
              <GitBranch aria-hidden="true" />
              Repository
            </a>
            <button className="button button--secondary" onClick={() => navigate("dashboard", onNavigate)}>
              <Activity aria-hidden="true" />
              Demo dashboard
            </button>
          </div>
        </div>

        <div className="signal-board" aria-label="Latest benchmark summary">
          <ul className="signal-board__stats" aria-label="Benchmark metrics">
            <li className="signal-board__stat">
              <span className="signal-board__stat-value">{formatNumber(httpBenchmark.targets.moonlight.requests_per_second, 0)}</span>
              <span className="signal-board__stat-description">HTTP requests/sec</span>
            </li>
            <li className="signal-board__stat">
              <span className="signal-board__stat-value">{formatMs(httpBenchmark.targets.moonlight.latency_ms.p95)}</span>
              <span className="signal-board__stat-description">HTTP p95 ms</span>
            </li>
            <li className="signal-board__stat">
              <span className="signal-board__stat-value">{formatMs(preferredCli.comparison.case_latency_ms.p95)}</span>
              <span className="signal-board__stat-description">
                CLI {preferredCli.name === "moonlight-argv" ? "argv " : ""}p95 ms/case
              </span>
            </li>
          </ul>
          <div className="pipeline" aria-hidden="true">
            <span>primary</span>
            <span>candidate</span>
            <span>secondary</span>
            <span>classify</span>
          </div>
        </div>
      </section>

      <section className="section section--split" id="repository">
        <div>
          <p className="eyebrow">Repository</p>
          <h2>One core comparer, multiple adapters, one inspection UI.</h2>
        </div>
        <div className="copy">
          <ul className="feature-list">
            <li><strong>moonlight-core</strong><span>Shared comparison, diffing, classification, and JSONL storage primitives.</span></li>
            <li><strong>moonlight-http</strong><span>An Axum proxy that shadows HTTP traffic to reference and candidate services.</span></li>
            <li><strong>moonlight</strong><span>A command runner for direct comparisons and batch command-output suites.</span></li>
            <li><strong>moonlight-ui</strong><span>A Vite admin UI for inspecting comparison runs and configuration.</span></li>
          </ul>
        </div>
      </section>

      <section className="section section--split" id="references">
        <div>
          <p className="eyebrow">Reference noise</p>
          <h2>Secondary references turn instability into signal.</h2>
        </div>
        <div className="copy">
          <p>
            Primary and secondary references expose unstable reference behavior such as timestamps, generated IDs, host-specific headers, and randomized ordering. Candidate differences are treated as suspicious only when they differ from stable reference behavior.
          </p>
        </div>
      </section>

      <BenchmarkSection
        title="HTTP Benchmark"
        generatedAt={httpBenchmark.generated_at}
        details={`${httpBenchmark.config.requests} requests, concurrency ${httpBenchmark.config.concurrency}, ${httpBenchmark.config.endpoints.length} endpoints`}
      >
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Target</th>
                <th>Requests</th>
                <th>Success</th>
                <th>Errors</th>
                <th>Req/s</th>
                <th>p50 ms</th>
                <th>p95 ms</th>
                <th>p99 ms</th>
                <th>Mean ms</th>
                <th>Max ms</th>
              </tr>
            </thead>
            <tbody>
              {httpTargets.map((target) => (
                <HttpBenchmarkRow key={target.name} target={target} />
              ))}
            </tbody>
          </table>
        </div>
        <div className="validity-strip">
          {httpBenchmark.validity.map((entry) => (
            <span key={entry.endpoint} className={`validity ${entry.match ? "match" : "target_error"}`}>
              {entry.endpoint}: {entry.match ? "match" : `${entry.mismatches.length} mismatches`}
            </span>
          ))}
        </div>
      </BenchmarkSection>

      <BenchmarkSection
        title="CLI Benchmark"
        generatedAt={cliBenchmark.generated_at}
        details={`${cliBenchmark.config.comparison_runs} suite runs, ${cliBenchmark.config.comparison_cases} cases/run, ${usingPublishedCliEvidence ? "published exact-head evidence" : "committed fallback"}, rustc ${cliBenchmark.environment.rustc}`}
      >
        {argvP95Reduction != null && argvP95 != null && shellP95 != null ? (
          <div className="fast-path-summary" aria-label="Direct argv performance summary">
            <strong>Direct argv is the preferred fast path.</strong>
            <span>
              {formatMs(argvP95)} ms/case p95 versus {formatMs(shellP95)} ms/case through shell commands — {formatNumber(argvP95Reduction, 1)}% lower latency in this run.
            </span>
          </div>
        ) : null}
        <CliPerformanceHistory history={cliHistory} />
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Target</th>
                <th>Status</th>
                <th>Suite runs</th>
                <th>Cases/run</th>
                <th>Targets/case</th>
                <th>Total cases</th>
                <th>Total target runs</th>
                <th>Suite p50 ms</th>
                <th>Suite p95 ms</th>
                <th>Per-case p50 ms</th>
                <th>Per-case p95 ms</th>
                <th>Per-target p50 ms</th>
                <th>Per-target p95 ms</th>
                <th>Version/Reason</th>
              </tr>
            </thead>
            <tbody>
              {cliTools.map(([name, comparison]) => (
                <CliBenchmarkRow key={name} name={name} comparison={comparison} />
              ))}
            </tbody>
          </table>
        </div>
        <div className="validity-strip">
          {Object.entries(cliBenchmark.scenarios).map(([scenario, result]) => (
            <span key={scenario} className={`validity ${result.validation_errors.length === 0 ? "match" : "target_error"}`}>
              {scenario}: {JSON.stringify(result.classifications)}
            </span>
          ))}
        </div>
      </BenchmarkSection>
    </section>
  );
}
