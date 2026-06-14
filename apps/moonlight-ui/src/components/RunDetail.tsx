import type { ComparisonRun, ComparisonRunListItem } from "../types";
import { labelFor, runTitle } from "../utils/run";
import { DiffViewer } from "./DiffViewer";
import { TargetCard } from "./TargetCard";

type RunDetailProps = {
  run: ComparisonRun | null;
  fallback: ComparisonRunListItem | null;
};

export function RunDetail({ run, fallback }: RunDetailProps) {
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
