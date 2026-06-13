import type { AppConfig, RequestListItem, RequestRecord, StatsSummary } from "./types";

const API_BASE = import.meta.env.VITE_SHADOWDIFF_API_URL ?? "";

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`);
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export const api = {
  config: () => getJson<AppConfig>("/api/config"),
  requests: () => getJson<RequestListItem[]>("/api/requests"),
  request: (id: string) => getJson<RequestRecord>(`/api/requests/${id}`),
  stats: () => getJson<StatsSummary>("/api/stats")
};
