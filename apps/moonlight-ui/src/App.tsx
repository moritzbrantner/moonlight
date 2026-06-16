import { useEffect, useMemo, useState } from "react";
import { api, type RunFilters } from "./api";
import { DashboardPage } from "./components/DashboardPage";
import { OverviewPage } from "./components/OverviewPage";
import { Topbar } from "./components/Topbar";
import { currentPage, type Page } from "./navigation";
import type { AppConfig, ComparisonRun, ComparisonRunListItem, RunReviewState, StatsSummary } from "./types";

export function App() {
  const [page, setPage] = useState(() => currentPage());
  const [stats, setStats] = useState<StatsSummary | null>(null);
  const [runs, setRuns] = useState<ComparisonRunListItem[]>([]);
  const [runTotal, setRunTotal] = useState(0);
  const [nextRunOffset, setNextRunOffset] = useState<number | null>(null);
  const [filters, setFilters] = useState<RunFilters>({ limit: 25 });
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selected, setSelected] = useState<ComparisonRun | null>(null);
  const [review, setReview] = useState<RunReviewState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  async function refresh() {
    setError(null);
    try {
      const [nextStats, nextRunPage, nextConfig] = await Promise.all([
        api.stats(),
        api.runs({ ...filters, offset: 0 }),
        api.config()
      ]);
      setStats(nextStats);
      const pageItems = Array.isArray(nextRunPage) ? nextRunPage : nextRunPage.items;
      setRuns(pageItems);
      setRunTotal(Array.isArray(nextRunPage) ? pageItems.length : nextRunPage.total);
      setNextRunOffset(Array.isArray(nextRunPage) ? null : nextRunPage.next_offset);
      setConfig(nextConfig);
      if (!selectedId && pageItems[0]) {
        setSelectedId(pageItems[0].id);
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
  }, [page, filters]);

  useEffect(() => {
    if (!selectedId) {
      setSelected(null);
      setReview(null);
      return;
    }
    Promise.all([
      api.run(selectedId),
      api.review?.(selectedId) ?? Promise.resolve({
        run_id: selectedId,
        status: "new" as const,
        note: null,
        tags: [],
        updated_at: new Date().toISOString()
      })
    ])
      .then(([run, review]) => {
        setSelected(run);
        setReview(review);
      })
      .catch((err: unknown) => {
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

  async function loadMoreRuns() {
    if (nextRunOffset === null) return;
    try {
      const page = await api.runs({ ...filters, offset: nextRunOffset });
      const pageItems = Array.isArray(page) ? page : page.items;
      setRuns((current) => [...current, ...pageItems]);
      setRunTotal(Array.isArray(page) ? pageItems.length : page.total);
      setNextRunOffset(Array.isArray(page) ? null : page.next_offset);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load more runs");
    }
  }

  async function updateReview(update: { status: RunReviewState["status"]; note?: string | null; tags?: string[] }) {
    if (!selectedId) return;
    try {
      const nextReview = await (api.updateReview?.(selectedId, update) ?? Promise.resolve({
        run_id: selectedId,
        status: update.status,
        note: update.note ?? null,
        tags: update.tags ?? [],
        updated_at: new Date().toISOString()
      }));
      setReview(nextReview);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update review");
    }
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
          onFiltersChange={setFilters}
          onLoadMoreRuns={loadMoreRuns}
          onUpdateReview={updateReview}
          review={review}
          runs={runs}
          runTotal={runTotal}
          selected={selected}
          selectedFromList={selectedFromList}
          selectedId={selectedId}
          stats={stats}
        />
      )}
    </main>
  );
}
