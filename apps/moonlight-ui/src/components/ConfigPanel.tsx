import { Settings } from "lucide-react";
import type { AppConfig } from "../types";

type ConfigPanelProps = {
  config: AppConfig | null;
};

export function ConfigPanel({ config }: ConfigPanelProps) {
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
          <dt>Request limit</dt>
          <dd>{config.max_request_body_bytes} bytes</dd>
          <dt>Redacted JSON</dt>
          <dd>{config.redact_json_paths.length ? config.redact_json_paths.join(", ") : "none"}</dd>
          <dt>Redacted query</dt>
          <dd>{config.redact_query_params.join(", ")}</dd>
          <dt>Ignored JSON</dt>
          <dd>{config.ignore_json_paths.join(", ")}</dd>
          <dt>Ignored headers</dt>
          <dd>{config.ignore_headers.join(", ")}</dd>
          <dt>CORS</dt>
          <dd>{config.cors_origins.join(", ")}</dd>
          <dt>Retention</dt>
          <dd>
            {config.retention_max_runs ?? "unbounded"} runs / {config.retention_max_bytes ?? "unbounded"} bytes
          </dd>
          <dt>Stderr</dt>
          <dd>{config.ignore_stderr ? "ignored" : "compared"}</dd>
        </dl>
      ) : (
        <p className="empty">Unavailable</p>
      )}
    </section>
  );
}
