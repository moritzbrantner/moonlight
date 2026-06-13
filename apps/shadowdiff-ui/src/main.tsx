import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { Activity, AlertTriangle, CheckCircle2, Clock, RefreshCw, Settings } from "lucide-react";
import { api } from "./api";
import type { AppConfig, BackendCapture, Classification, DiffEntry, RequestListItem, RequestRecord, StatsSummary } from "./types";
import "./styles.css";

function App() {
  const [stats, setStats] = useState<StatsSummary | null>(null);
  const [requests, setRequests] = useState<RequestListItem[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selected, setSelected] = useState<RequestRecord | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  async function refresh() {
    setError(null);
    try {
      const [nextStats, nextRequests, nextConfig] = await Promise.all([
        api.stats(),
        api.requests(),
        api.config()
      ]);
      setStats(nextStats);
      setRequests(nextRequests);
      setConfig(nextConfig);
      if (!selectedId && nextRequests[0]) {
        setSelectedId(nextRequests[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load Shadowdiff data");
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
    api.request(selectedId).then(setSelected).catch((err: unknown) => {
      setError(err instanceof Error ? err.message : "Failed to load request");
    });
  }, [selectedId]);

  const selectedFromList = useMemo(
    () => requests.find((request) => request.id === selectedId) ?? null,
    [requests, selectedId]
  );

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <h1>Shadowdiff</h1>
          <p>Shadow traffic comparison for primary, secondary, and candidate services.</p>
        </div>
        <button className="icon-button" onClick={() => void refresh()} title="Refresh data">
          <RefreshCw size={18} />
        </button>
      </header>

      {error && <div className="banner">{error}</div>}
      {loading && <div className="banner muted">Loading admin API data...</div>}

      <section className="dashboard">
        <Metric label="Total" value={stats?.total_requests ?? 0} icon={<Activity size={18} />} />
        <Metric label="Matches" value={stats?.matches ?? 0} icon={<CheckCircle2 size={18} />} />
        <Metric label="Diffs" value={(stats?.candidate_diffs ?? 0) + (stats?.candidate_diff_with_noise ?? 0)} icon={<AlertTriangle size={18} />} />
        <Metric label="Noise" value={stats?.noise ?? 0} icon={<Activity size={18} />} />
        <Metric label="Errors" value={stats?.backend_errors ?? 0} icon={<AlertTriangle size={18} />} />
        <Metric label="Primary avg" value={`${(stats?.latency.primary_avg_ms ?? 0).toFixed(1)} ms`} icon={<Clock size={18} />} />
      </section>

      <section className="workspace">
        <RequestList requests={requests} selectedId={selectedId} onSelect={setSelectedId} />
        <RequestDetail request={selected} fallback={selectedFromList} />
        <aside className="side-panel">
          <ConfigPanel config={config} />
          <LatencyPanel request={selected} />
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

function RequestList({ requests, selectedId, onSelect }: { requests: RequestListItem[]; selectedId: string | null; onSelect: (id: string) => void }) {
  return (
    <section className="request-list" aria-label="Request history">
      <div className="section-heading">
        <h2>Requests</h2>
        <span>{requests.length}</span>
      </div>
      <div className="list-scroll">
        {requests.map((request) => (
          <button
            key={request.id}
            className={`request-row ${request.id === selectedId ? "selected" : ""}`}
            onClick={() => onSelect(request.id)}
          >
            <span className="method">{request.method}</span>
            <span className="path">{request.path}{request.query ? `?${request.query}` : ""}</span>
            <span className={`pill ${request.classification}`}>{labelFor(request.classification)}</span>
            <span className="status">{request.primary_status ?? "ERR"} / {request.candidate_status ?? "-"}</span>
          </button>
        ))}
        {requests.length === 0 && <p className="empty">No traffic recorded yet.</p>}
      </div>
    </section>
  );
}

function RequestDetail({ request, fallback }: { request: RequestRecord | null; fallback: RequestListItem | null }) {
  if (!request) {
    return (
      <section className="detail-panel">
        <div className="section-heading">
          <h2>{fallback ? `${fallback.method} ${fallback.path}` : "Request detail"}</h2>
        </div>
        <p className="empty">Select a request to inspect responses and diffs.</p>
      </section>
    );
  }

  return (
    <section className="detail-panel">
      <div className="detail-title">
        <div>
          <span className="method large">{request.method}</span>
          <h2>{request.path}{request.query ? `?${request.query}` : ""}</h2>
          <p>{new Date(request.timestamp).toLocaleString()}</p>
        </div>
        <span className={`pill ${request.comparison.classification}`}>{labelFor(request.comparison.classification)}</span>
      </div>

      <div className="backend-grid">
        <BackendCard title="Primary" backend={request.primary} />
        <BackendCard title="Candidate" backend={request.candidate} />
        <BackendCard title="Secondary" backend={request.secondary} />
      </div>

      <DiffViewer title="Noise-filtered diff" diffs={request.comparison.noise_filtered_diffs} />
      <DiffViewer title="Raw candidate diff" diffs={request.comparison.raw_candidate_diffs} />
      <DiffViewer title="Reference noise" diffs={request.comparison.reference_noise} />
    </section>
  );
}

function BackendCard({ title, backend }: { title: string; backend: BackendCapture | null }) {
  return (
    <div className="backend-card">
      <h3>{title}</h3>
      {backend ? (
        <>
          <dl>
            <dt>Status</dt>
            <dd>{backend.status ?? "error"}</dd>
            <dt>Latency</dt>
            <dd>{backend.latency_ms} ms</dd>
            <dt>Body</dt>
            <dd>{backend.body.size_bytes} bytes</dd>
          </dl>
          {backend.error && <p className="error-text">{backend.error}</p>}
          <pre>{backend.body.preview || "(empty)"}</pre>
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
          <dd>{config.enable_candidate ? config.candidate_url : "disabled"}</dd>
          <dt>Secondary</dt>
          <dd>{config.enable_secondary ? config.secondary_url : "disabled"}</dd>
          <dt>Capture</dt>
          <dd>{config.max_body_capture_bytes} bytes</dd>
          <dt>Ignored JSON</dt>
          <dd>{config.ignored_json_paths.join(", ")}</dd>
          <dt>Ignored headers</dt>
          <dd>{config.ignored_headers.join(", ")}</dd>
        </dl>
      ) : (
        <p className="empty">Unavailable</p>
      )}
    </section>
  );
}

function LatencyPanel({ request }: { request: RequestRecord | null }) {
  return (
    <section className="config-panel">
      <div className="section-heading">
        <h2>Latency</h2>
        <Clock size={18} />
      </div>
      {request ? (
        <dl>
          <dt>Primary</dt>
          <dd>{request.primary.latency_ms} ms</dd>
          <dt>Candidate</dt>
          <dd>{request.candidate ? `${request.candidate.latency_ms} ms` : "disabled"}</dd>
          <dt>Secondary</dt>
          <dd>{request.secondary ? `${request.secondary.latency_ms} ms` : "disabled"}</dd>
        </dl>
      ) : (
        <p className="empty">Select a request.</p>
      )}
    </section>
  );
}

function labelFor(classification: Classification) {
  return classification.replace(/_/g, " ");
}

createRoot(document.getElementById("root")!).render(<App />);
