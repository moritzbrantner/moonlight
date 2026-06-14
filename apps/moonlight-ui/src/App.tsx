import { useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { DashboardPage } from "./components/DashboardPage";
import { OverviewPage } from "./components/OverviewPage";
import { Topbar } from "./components/Topbar";
import { currentPage, type Page } from "./navigation";
import type { AppConfig, ComparisonRun, ComparisonRunListItem, StatsSummary } from "./types";

export function App() {
  const [page, setPage] = useState(() => currentPage());
  const [stats, setStats] = useState<StatsSummary | null>(null);
  const [runs, setRuns] = useState<ComparisonRunListItem[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selected, setSelected] = useState<ComparisonRun | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  async function refresh() {
    setError(null);
    try {
      const [nextStats, nextRuns, nextConfig] = await Promise.all([
        api.stats(),
        api.runs(),
        api.config()
      ]);
      setStats(nextStats);
      setRuns(nextRuns);
      setConfig(nextConfig);
      if (!selectedId && nextRuns[0]) {
        setSelectedId(nextRuns[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load Moonlight data");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const handlePopState = () => setPage(currentPage());
    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  useEffect(() => {
    if (page !== "dashboard") {
      setLoading(false);
      return;
    }
    void refresh();
    const timer = window.setInterval(() => void refresh(), 3500);
    return () => window.clearInterval(timer);
  }, [page]);

  useEffect(() => {
    if (!selectedId) {
      setSelected(null);
      return;
    }
    api.run(selectedId).then(setSelected).catch((err: unknown) => {
      setError(err instanceof Error ? err.message : "Failed to load run");
    });
  }, [selectedId]);

  const selectedFromList = useMemo(
    () => runs.find((run) => run.id === selectedId) ?? null,
    [runs, selectedId]
  );

  function setPageFromChild(nextPage: Page) {
    setPage(nextPage);
  }

  return (
    <main className="app-shell">
      <Topbar page={page} onNavigate={setPage} onRefresh={refresh} />
      {page === "overview" ? (
        <OverviewPage onNavigate={setPageFromChild} />
      ) : (
        <DashboardPage
          config={config}
          error={error}
          loading={loading}
          onSelectRun={setSelectedId}
          runs={runs}
          selected={selected}
          selectedFromList={selectedFromList}
          selectedId={selectedId}
          stats={stats}
        />
      )}
    </main>
  );
}
