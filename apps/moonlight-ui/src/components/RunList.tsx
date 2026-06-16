import { useState } from "react";
import type { RunFilters } from "../api";
import type { ComparisonRunListItem } from "../types";
import { labelFor, runTitle } from "../utils/run";

type RunListProps = {
  runs: ComparisonRunListItem[];
  runTotal?: number;
  selectedId: string | null;
  onFiltersChange?: (filters: RunFilters) => void;
  onLoadMore?: () => void;
  onSelect: (id: string) => void;
};

export function RunList({ runs, runTotal = runs.length, selectedId, onFiltersChange = () => {}, onLoadMore = () => {}, onSelect }: RunListProps) {
  const [query, setQuery] = useState("");
  const [classification, setClassification] = useState<RunFilters["classification"]>("");
  const [adapter, setAdapter] = useState<RunFilters["adapter"]>("");
  const [status, setStatus] = useState("");

  function applyFilters(next: Partial<RunFilters>) {
    const merged = {
      classification,
      adapter,
      q: query,
      status,
      limit: 25,
      ...next
    };
    onFiltersChange(merged);
  }

  return (
    <section className="request-list" aria-label="Run history">
      <div className="section-heading">
        <h2>Runs</h2>
        <span><strong>{runs.length}</strong> / {runTotal}</span>
      </div>
      <div className="run-filters">
        <input
          aria-label="Search runs"
          placeholder="Search"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            applyFilters({ q: event.target.value });
          }}
        />
        <select
          aria-label="Filter by classification"
          value={classification}
          onChange={(event) => {
            const value = event.target.value as RunFilters["classification"];
            setClassification(value);
            applyFilters({ classification: value });
          }}
        >
          <option value="">All results</option>
          <option value="match">Match</option>
          <option value="suspicious_difference">Suspicious diff</option>
          <option value="reference_noise">Noise</option>
          <option value="suspicious_with_noise">Mixed</option>
          <option value="target_error">Errors</option>
        </select>
        <select
          aria-label="Filter by adapter"
          value={adapter}
          onChange={(event) => {
            const value = event.target.value as RunFilters["adapter"];
            setAdapter(value);
            applyFilters({ adapter: value });
          }}
        >
          <option value="">All adapters</option>
          <option value="http">HTTP</option>
          <option value="cli">CLI</option>
        </select>
        <input
          aria-label="Filter by status"
          inputMode="numeric"
          placeholder="Status"
          value={status}
          onChange={(event) => {
            setStatus(event.target.value);
            applyFilters({ status: event.target.value });
          }}
        />
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
        {runs.length < runTotal && (
          <button className="load-more" onClick={onLoadMore}>
            Load more
          </button>
        )}
      </div>
    </section>
  );
}
