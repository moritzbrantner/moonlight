import type { ComparisonRunListItem } from "../types";
import { labelFor, runTitle } from "../utils/run";

type RunListProps = {
  runs: ComparisonRunListItem[];
  selectedId: string | null;
  onSelect: (id: string) => void;
};

export function RunList({ runs, selectedId, onSelect }: RunListProps) {
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
