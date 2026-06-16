import { Activity, AlertTriangle, CheckCircle2, Clock } from "lucide-react";
import type { RunFilters } from "../api";
import { usesDemoData } from "../api";
import type { AppConfig, ComparisonRun, ComparisonRunListItem, RunReviewState, StatsSummary } from "../types";
import { ConfigPanel } from "./ConfigPanel";
import { LatencyPanel } from "./LatencyPanel";
import { Metric } from "./Metric";
import { RunDetail } from "./RunDetail";
import { RunList } from "./RunList";

type DashboardPageProps = {
  config: AppConfig | null;
  error: string | null;
  loading: boolean;
  onFiltersChange?: (filters: RunFilters) => void;
  onLoadMoreRuns?: () => void;
  onSelectRun: (id: string) => void;
  onUpdateReview?: (update: { status: RunReviewState["status"]; note?: string | null; tags?: string[] }) => void;
  review?: RunReviewState | null;
  runs: ComparisonRunListItem[];
  runTotal?: number;
  selected: ComparisonRun | null;
  selectedFromList: ComparisonRunListItem | null;
  selectedId: string | null;
  stats: StatsSummary | null;
};

export function DashboardPage({
  config,
  error,
  loading,
  onFiltersChange = () => {},
  onLoadMoreRuns = () => {},
  onSelectRun,
  onUpdateReview = () => {},
  review = null,
  runs,
  runTotal = runs.length,
  selected,
  selectedFromList,
  selectedId,
  stats
}: DashboardPageProps) {
  return (
    <>
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
        <RunList
          runs={runs}
          runTotal={runTotal}
          selectedId={selectedId}
          onFiltersChange={onFiltersChange}
          onLoadMore={onLoadMoreRuns}
          onSelect={onSelectRun}
        />
        <RunDetail run={selected} fallback={selectedFromList} review={review} onUpdateReview={onUpdateReview} />
        <aside className="side-panel">
          <ConfigPanel config={config} />
          <LatencyPanel run={selected} />
        </aside>
      </section>
    </>
  );
}
