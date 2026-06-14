import { Activity, Github } from "lucide-react";
import { cliBenchmark, httpBenchmark, type CliToolComparison } from "../benchmarkData";
import { navigate, type Page } from "../navigation";
import { formatMs, formatNumber } from "../utils/format";
import { BenchmarkSection } from "./BenchmarkSection";
import { CliBenchmarkRow } from "./CliBenchmarkRow";
import { HttpBenchmarkRow } from "./HttpBenchmarkRow";

type OverviewPageProps = {
  onNavigate: (page: Page) => void;
};

export function OverviewPage({ onNavigate }: OverviewPageProps) {
  const httpTargets = ["moonlight", "diffy_b", "diffy_c"]
    .map((key) => httpBenchmark.targets[key])
    .filter(Boolean);
  const configuredCliTargets = cliBenchmark.config.targets?.length
    ? cliBenchmark.config.targets
    : Object.keys(cliBenchmark.comparisons);
  const cliTools = configuredCliTargets
    .map((key) => [key, cliBenchmark.comparisons[key]] as const)
    .filter((entry): entry is readonly [string, CliToolComparison] => Boolean(entry[1]));

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
              <Github aria-hidden="true" />
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
              <span className="signal-board__stat-value">{formatMs(cliBenchmark.comparisons.moonlight.case_latency_ms.p95)}</span>
              <span className="signal-board__stat-description">CLI p95 ms/case</span>
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
          <h2>One core comparer, two adapters, one inspection UI.</h2>
        </div>
        <div className="copy">
          <ul className="feature-list">
            <li><strong>moonlight-core</strong><span>Shared comparison, diffing, classification, and JSONL storage primitives.</span></li>
            <li><strong>moonlight-http</strong><span>An Axum proxy that shadows HTTP traffic to reference and candidate services.</span></li>
            <li><strong>moonlight-cli</strong><span>A command runner for direct comparisons and batch command-output suites.</span></li>
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
        details={`${cliBenchmark.config.comparison_runs} suite runs, ${cliBenchmark.config.comparison_cases} cases/run, rustc ${cliBenchmark.environment.rustc}`}
      >
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
