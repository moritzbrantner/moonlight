import type { AppConfig, ComparisonRun, ComparisonRunListItem, StatsSummary } from "./types";
import { demoConfig, demoRunList, demoRuns, demoStats } from "./demoData";

const API_BASE = import.meta.env.VITE_MOONLIGHT_API_URL ?? "";
export const usesDemoData = import.meta.env.VITE_MOONLIGHT_DEMO === "true";

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`);
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export const api = {
  config: () => usesDemoData ? Promise.resolve(demoConfig) : getJson<AppConfig>("/api/config"),
  runs: () => usesDemoData ? Promise.resolve(demoRunList) : getJson<ComparisonRunListItem[]>("/api/runs"),
  run: (id: string) => {
    if (usesDemoData) {
      const run = demoRuns.find((candidate) => candidate.id === id);
      return run ? Promise.resolve(run) : Promise.reject(new Error(`Demo run ${id} not found`));
    }
    return getJson<ComparisonRun>(`/api/runs/${id}`);
  },
  stats: () => usesDemoData ? Promise.resolve(demoStats) : getJson<StatsSummary>("/api/stats")
};
