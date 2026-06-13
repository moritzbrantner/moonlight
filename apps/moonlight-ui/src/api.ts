import type { AppConfig, ComparisonRun, ComparisonRunListItem, StatsSummary } from "./types";

const API_BASE = import.meta.env.VITE_MOONLIGHT_API_URL ?? "";

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`);
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export const api = {
  config: () => getJson<AppConfig>("/api/config"),
  runs: () => getJson<ComparisonRunListItem[]>("/api/runs"),
  run: (id: string) => getJson<ComparisonRun>(`/api/runs/${id}`),
  stats: () => getJson<StatsSummary>("/api/stats")
};
