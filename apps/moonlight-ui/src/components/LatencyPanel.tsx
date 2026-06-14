import { Clock } from "lucide-react";
import type { ComparisonRun } from "../types";

type LatencyPanelProps = {
  run: ComparisonRun | null;
};

export function LatencyPanel({ run }: LatencyPanelProps) {
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
