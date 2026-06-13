import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { Activity, AlertTriangle, CheckCircle2, Clock, Github, RefreshCw, Settings } from "lucide-react";
import { api, usesDemoData } from "./api";
import { cliBenchmark, httpBenchmark, type CliToolComparison, type HttpBenchmarkTarget, type PercentileSummary } from "./benchmarkData";
import type { AppConfig, Classification, ComparisonRun, ComparisonRunListItem, DiffEntry, RunInput, StatsSummary, TargetObservation } from "./types";
import "./styles.css";

function App() {
  const [page, setPage] = useState(() => currentPage());
  const [stats, setStats] = useState<StatsSummary | null>(null);
  const [runs, setRuns] = useState<ComparisonRunListItem[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selected, setSelected] = useState<ComparisonRun | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  async function refresh() {
    setError(null);
    try {
      const [nextStats, nextRuns, nextConfig] = await Promise.all([
        api.stats(),
        api.runs(),
        api.config()
      ]);
      setStats(nextStats);
      setRuns(nextRuns);
      setConfig(nextConfig);
      if (!selectedId && nextRuns[0]) {
        setSelectedId(nextRuns[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load Moonlight data");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const handlePopState = () => setPage(currentPage());
    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  useEffect(() => {
    if (page !== "dashboard") {
      setLoading(false);
      return;
    }
    void refresh();
    const timer = window.setInterval(() => void refresh(), 3500);
    return () => window.clearInterval(timer);
  }, [page]);

  useEffect(() => {
    if (!selectedId) {
      setSelected(null);
      return;
    }
    api.run(selectedId).then(setSelected).catch((err: unknown) => {
      setError(err instanceof Error ? err.message : "Failed to load run");
    });
  }, [selectedId]);

  const selectedFromList = useMemo(
    () => runs.find((run) => run.id === selectedId) ?? null,
    [runs, selectedId]
  );

  return (
    <main className="app-shell">
      <header className="topbar">
        <button className="brand" onClick={() => navigate("overview", setPage)} aria-label="Moonlight overview">
          <span className="brand__mark" aria-hidden="true">ML</span>
          <span>Moonlight</span>
        </button>
        <nav className="top-actions" aria-label="Pages">
          <button className={`nav-button ${page === "overview" ? "active" : ""}`} onClick={() => navigate("overview", setPage)}>
            Overview
          </button>
          <button className={`nav-button ${page === "dashboard" ? "active" : ""}`} onClick={() => navigate("dashboard", setPage)}>
            Dashboard
          </button>
          {page === "dashboard" && (
            <button className="icon-button" onClick={() => void refresh()} title="Refresh data">
              <RefreshCw size={18} />
            </button>
          )}
        </nav>
      </header>

      {page === "overview" ? (
        <OverviewPage onNavigate={setPage} />
      ) : (
        <>
      {error && <div className="banner">{error}</div>}
      {usesDemoData && <div className="banner muted">Demo data for the GitHub Pages example.</div>}
      {loading && <div className="banner muted">Loading admin API data...</div>}

      <section className="dashboard">
        <Metric label="Total" value={stats?.total_runs ?? 0} icon={<Activity size={18} />} />
        <Metric label="Matches" value={stats?.matches ?? 0} icon={<CheckCircle2 size={18} />} />
        <Metric label="Suspicious" value={(stats?.suspicious_differences ?? 0) + (stats?.suspicious_with_noise ?? 0)} icon={<AlertTriangle size={18} />} />
        <Metric label="Noise" value={stats?.reference_noise ?? 0} icon={<Activity size={18} />} />
        <Metric label="Errors" value={stats?.target_errors ?? 0} icon={<AlertTriangle size={18} />} />
        <Metric label="Primary avg" value={`${(stats?.latency.primary_avg_ms ?? 0).toFixed(1)} ms`} icon={<Clock size={18} />} />
      </section>

      <section className="workspace">
        <RunList runs={runs} selectedId={selectedId} onSelect={setSelectedId} />
        <RunDetail run={selected} fallback={selectedFromList} />
        <aside className="side-panel">
          <ConfigPanel config={config} />
          <LatencyPanel run={selected} />
        </aside>
      </section>
        </>
      )}
    </main>
  );
}

function OverviewPage({ onNavigate }: { onNavigate: (page: Page) => void }) {
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

function BenchmarkSection({ title, generatedAt, details, children }: { title: string; generatedAt: string; details: string; children: React.ReactNode }) {
  return (
    <section className="benchmark-section">
      <div className="benchmark-heading">
        <div>
          <h3>{title}</h3>
          <p>{details}</p>
        </div>
        <time dateTime={generatedAt}>{new Date(generatedAt).toLocaleString()}</time>
      </div>
      {children}
    </section>
  );
}

function HttpBenchmarkRow({ target }: { target: HttpBenchmarkTarget }) {
  return (
    <tr>
      <td>{target.name}</td>
      <td>{target.total_requests}</td>
      <td>{target.success_count}</td>
      <td>{target.error_count}</td>
      <td>{formatNumber(target.requests_per_second)}</td>
      <LatencyCells latency={target.latency_ms} />
    </tr>
  );
}

function CliBenchmarkRow({ name, comparison }: { name: string; comparison: CliToolComparison }) {
  const targetInvocationsPerCase =
    comparison.target_invocations_per_case ?? (name.startsWith("moonlight") ? 2 : 1);
  const totalTargetInvocations =
    comparison.total_target_invocations ?? comparison.total_cases * targetInvocationsPerCase;
  const targetP50 =
    comparison.target_invocation_latency_ms?.p50 ??
    divideMs(comparison.case_latency_ms.p50, targetInvocationsPerCase);
  const targetP95 =
    comparison.target_invocation_latency_ms?.p95 ??
    divideMs(comparison.case_latency_ms.p95, targetInvocationsPerCase);

  return (
    <tr>
      <td>{name}</td>
      <td>{comparison.status}</td>
      <td>{comparison.total_invocations}</td>
      <td>{comparison.cases_per_invocation}</td>
      <td>{targetInvocationsPerCase}</td>
      <td>{comparison.total_cases}</td>
      <td>{totalTargetInvocations}</td>
      <td>{formatMs(comparison.latency_ms.p50)}</td>
      <td>{formatMs(comparison.latency_ms.p95)}</td>
      <td>{formatMs(comparison.case_latency_ms.p50)}</td>
      <td>{formatMs(comparison.case_latency_ms.p95)}</td>
      <td>{formatMs(targetP50)}</td>
      <td>{formatMs(targetP95)}</td>
      <td>{comparison.reason ?? comparison.version ?? ""}</td>
    </tr>
  );
}

function divideMs(value: number | null, denominator: number) {
  return value === null ? null : value / denominator;
}

function LatencyCells({ latency }: { latency: PercentileSummary }) {
  return (
    <>
      <td>{formatMs(latency.p50)}</td>
      <td>{formatMs(latency.p95)}</td>
      <td>{formatMs(latency.p99)}</td>
      <td>{formatMs(latency.mean)}</td>
      <td>{formatMs(latency.max)}</td>
    </>
  );
}

function Metric({ label, value, icon }: { label: string; value: React.ReactNode; icon: React.ReactNode }) {
  return (
    <div className="metric">
      <div className="metric-icon">{icon}</div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function RunList({ runs, selectedId, onSelect }: { runs: ComparisonRunListItem[]; selectedId: string | null; onSelect: (id: string) => void }) {
  return (
    <section className="request-list" aria-label="Run history">
      <div className="section-heading">
        <h2>Runs</h2>
        <span>{runs.length}</span>
      </div>
      <div className="list-scroll">
        {runs.map((run) => (
          <button
            key={run.id}
            className={`request-row ${run.id === selectedId ? "selected" : ""}`}
            onClick={() => onSelect(run.id)}
          >
            <span className="method">{run.adapter.toUpperCase()}</span>
            <span className="path">{runTitle(run.input)}</span>
            <span className={`pill ${run.classification}`}>{labelFor(run.classification)}</span>
            <span className="status">{run.primary_status ?? "ERR"} / {run.candidate_status ?? "ERR"}</span>
          </button>
        ))}
        {runs.length === 0 && <p className="empty">No runs recorded yet.</p>}
      </div>
    </section>
  );
}

function RunDetail({ run, fallback }: { run: ComparisonRun | null; fallback: ComparisonRunListItem | null }) {
  if (!run) {
    return (
      <section className="detail-panel">
        <div className="section-heading">
          <h2>{fallback ? runTitle(fallback.input) : "Run detail"}</h2>
        </div>
        <p className="empty">Select a run to inspect target observations and diffs.</p>
      </section>
    );
  }

  return (
    <section className="detail-panel">
      <div className="detail-title">
        <div>
          <span className="method large">{run.adapter.toUpperCase()}</span>
          <h2>{runTitle(run.input)}</h2>
          <p>{new Date(run.timestamp).toLocaleString()}</p>
        </div>
        <span className={`pill ${run.comparison.classification}`}>{labelFor(run.comparison.classification)}</span>
      </div>

      <div className="target-grid">
        <TargetCard title="Primary Reference" target={run.primary} />
        <TargetCard title="Candidate" target={run.candidate} />
        <TargetCard title="Secondary Reference" target={run.secondary} />
      </div>

      <DiffViewer title="Noise-filtered diff" diffs={run.comparison.noise_filtered_diffs} />
      <DiffViewer title="Raw candidate diff" diffs={run.comparison.raw_candidate_diffs} />
      <DiffViewer title="Reference noise" diffs={run.comparison.reference_noise} />
    </section>
  );
}

function TargetCard({ title, target }: { title: string; target: TargetObservation | null }) {
  return (
    <div className="target-card">
      <h3>{title}</h3>
      {target ? (
        <>
          <dl>
            <dt>Status</dt>
            <dd>{target.status ?? "error"}</dd>
            <dt>Latency</dt>
            <dd>{target.latency_ms} ms</dd>
            <dt>Body</dt>
            <dd>{target.body.size_bytes} bytes</dd>
            {target.stderr && (
              <>
                <dt>Stderr</dt>
                <dd>{target.stderr.size_bytes} bytes</dd>
              </>
            )}
          </dl>
          {target.error && <p className="error-text">{target.error}</p>}
          <pre>{target.body.preview || "(empty)"}</pre>
          {target.stderr?.preview && <pre>{target.stderr.preview}</pre>}
        </>
      ) : (
        <p className="empty">Disabled</p>
      )}
    </div>
  );
}

function DiffViewer({ title, diffs }: { title: string; diffs: DiffEntry[] }) {
  return (
    <section className="diff-viewer">
      <div className="section-heading">
        <h3>{title}</h3>
        <span>{diffs.length}</span>
      </div>
      {diffs.length === 0 ? (
        <p className="empty">No entries.</p>
      ) : (
        <div className="diff-table">
          {diffs.map((diff, index) => (
            <div className="diff-row" key={`${diff.path}-${index}`}>
              <span>{diff.kind}</span>
              <strong>{diff.path}</strong>
              <code>P: {diff.primary ?? "-"}</code>
              <code>C: {diff.candidate ?? "-"}</code>
              <code>S: {diff.secondary ?? "-"}</code>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function ConfigPanel({ config }: { config: AppConfig | null }) {
  return (
    <section className="config-panel">
      <div className="section-heading">
        <h2>Config</h2>
        <Settings size={18} />
      </div>
      {config ? (
        <dl>
          <dt>Primary</dt>
          <dd>{config.primary_url}</dd>
          <dt>Candidate</dt>
          <dd>{config.candidate_url}</dd>
          <dt>Secondary</dt>
          <dd>{config.enable_secondary ? config.secondary_url : "disabled"}</dd>
          <dt>Return target</dt>
          <dd>{config.return_target}</dd>
          <dt>Fallback</dt>
          <dd>{config.return_fallback}</dd>
          <dt>Timing</dt>
          <dd>{config.response_timing}</dd>
          <dt>Capture</dt>
          <dd>{config.max_body_capture_bytes} bytes</dd>
          <dt>Ignored JSON</dt>
          <dd>{config.ignored_json_paths.join(", ")}</dd>
          <dt>Ignored headers</dt>
          <dd>{config.ignored_headers.join(", ")}</dd>
          <dt>Stderr</dt>
          <dd>{config.ignore_stderr ? "ignored" : "compared"}</dd>
        </dl>
      ) : (
        <p className="empty">Unavailable</p>
      )}
    </section>
  );
}

function LatencyPanel({ run }: { run: ComparisonRun | null }) {
  return (
    <section className="config-panel">
      <div className="section-heading">
        <h2>Latency</h2>
        <Clock size={18} />
      </div>
      {run ? (
        <dl>
          <dt>Primary</dt>
          <dd>{run.primary.latency_ms} ms</dd>
          <dt>Candidate</dt>
          <dd>{run.candidate.latency_ms} ms</dd>
          <dt>Secondary</dt>
          <dd>{run.secondary ? `${run.secondary.latency_ms} ms` : "disabled"}</dd>
        </dl>
      ) : (
        <p className="empty">Select a run.</p>
      )}
    </section>
  );
}

function runTitle(input: RunInput) {
  if ("method" in input) {
    return `${input.method} ${input.path}${input.query ? `?${input.query}` : ""}`;
  }
  return input.candidate_command;
}

function labelFor(classification: Classification) {
  return classification
    .split("_")
    .map((word) => word[0].toUpperCase() + word.slice(1))
    .join(" ");
}

type Page = "overview" | "dashboard";

function currentPage(): Page {
  return new URLSearchParams(window.location.search).get("page") === "overview" ? "overview" : "dashboard";
}

function navigate(page: Page, setPage: (page: Page) => void) {
  const url = new URL(window.location.href);
  if (page === "overview") {
    url.searchParams.set("page", "overview");
  } else {
    url.searchParams.delete("page");
  }
  window.history.pushState({}, "", url);
  setPage(page);
}

function formatMs(value: number | null) {
  return value === null ? "-" : value.toFixed(2);
}

function formatNumber(value: number, maximumFractionDigits = 2) {
  return new Intl.NumberFormat("en", { maximumFractionDigits }).format(value);
}

createRoot(document.getElementById("root")!).render(<App />);
