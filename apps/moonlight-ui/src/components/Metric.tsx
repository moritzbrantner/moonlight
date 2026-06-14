import type { ReactNode } from "react";

type MetricProps = {
  label: string;
  value: ReactNode;
  icon: ReactNode;
};

export function Metric({ label, value, icon }: MetricProps) {
  return (
    <div className="metric">
      <div className="metric-icon">{icon}</div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
