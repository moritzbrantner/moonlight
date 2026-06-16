import type { ComparisonRun, ComparisonRunListItem, RunReviewState } from "../types";
import { labelFor, runTitle } from "../utils/run";
import { DiffViewer } from "./DiffViewer";
import { TargetCard } from "./TargetCard";

type RunDetailProps = {
  run: ComparisonRun | null;
  fallback: ComparisonRunListItem | null;
  review?: RunReviewState | null;
  onUpdateReview?: (update: { status: RunReviewState["status"]; note?: string | null; tags?: string[] }) => void;
};

export function RunDetail({ run, fallback, review = null, onUpdateReview = () => {} }: RunDetailProps) {
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

      <div className="review-actions" aria-label="Review state">
        <span className="review-status">Review: {review?.status ?? "new"}</span>
        <button onClick={() => onUpdateReview({ status: "accepted", note: review?.note, tags: review?.tags ?? [] })}>Accept</button>
        <button onClick={() => onUpdateReview({ status: "ignored", note: review?.note, tags: review?.tags ?? [] })}>Ignore</button>
        <button onClick={() => onUpdateReview({ status: "fixed", note: review?.note, tags: review?.tags ?? [] })}>Fixed</button>
        <a href={`/api/runs/${run.id}/report?format=markdown`} target="_blank" rel="noreferrer">Markdown</a>
        <a href={`/api/runs/${run.id}/report?format=json`} target="_blank" rel="noreferrer">JSON</a>
      </div>

      <div className="review-note">
        <input
          aria-label="Review note"
          placeholder="Review note"
          value={review?.note ?? ""}
          onChange={(event) => onUpdateReview({ status: review?.status ?? "new", note: event.target.value, tags: review?.tags ?? [] })}
        />
      </div>

      <div className="target-grid">
        <TargetCard title="Primary Reference" target={run.primary} />
        <TargetCard title="Candidate" target={run.candidate} />
        <TargetCard title="Secondary Reference" target={run.secondary} />
      </div>

      <DiffViewer title="Noise-filtered diff" diffs={run.comparison.noise_filtered_diffs} />
      {run.comparison.reference_noise.length > 0 && (
        <div className="suggestions">
          {run.comparison.reference_noise.slice(0, 3).map((diff) => (
            <button
              key={`${diff.kind}-${diff.path}`}
              onClick={() => onUpdateReview({
                status: review?.status ?? "new",
                note: review?.note,
                tags: [...new Set([...(review?.tags ?? []), `ignore:${diff.path}`])]
              })}
            >
              Suggest ignore {diff.path}
            </button>
          ))}
        </div>
      )}
      <DiffViewer title="Raw candidate diff" diffs={run.comparison.raw_candidate_diffs} />
      <DiffViewer title="Reference noise" diffs={run.comparison.reference_noise} />
    </section>
  );
}
