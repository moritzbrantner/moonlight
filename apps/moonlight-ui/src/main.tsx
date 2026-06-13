import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { Activity, AlertTriangle, CheckCircle2, Clock, RefreshCw, Settings } from "lucide-react";
import { api, usesDemoData } from "./api";
import type { AppConfig, Classification, ComparisonRun, ComparisonRunListItem, DiffEntry, RunInput, StatsSummary, TargetObservation } from "./types";
import "./styles.css";

function App() {
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
    void refresh();
    const timer = window.setInterval(() => void refresh(), 3500);
    return () => window.clearInterval(timer);
  }, []);

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
        <div>
          <h1>Moonlight</h1>
          <p>Behavior comparison for primary and secondary references against candidate targets.</p>
        </div>
        <button className="icon-button" onClick={() => void refresh()} title="Refresh data">
          <RefreshCw size={18} />
        </button>
      </header>

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
    </main>
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

createRoot(document.getElementById("root")!).render(<App />);
