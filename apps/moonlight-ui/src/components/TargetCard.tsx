import type { TargetObservation } from "../types";

type TargetCardProps = {
  title: string;
  target: TargetObservation | null;
};

export function TargetCard({ title, target }: TargetCardProps) {
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
