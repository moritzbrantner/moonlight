import type {
  Adapter,
  AppConfig,
  Classification,
  ComparisonRun,
  ReviewUpdate,
  RunPage,
  RunReviewState,
  StatsSummary
} from "./types";
import { demoConfig, demoRunList, demoReviewStates, demoRuns, demoStats } from "./demoData";

const API_BASE = import.meta.env.VITE_MOONLIGHT_API_URL ?? "";
const ADMIN_TOKEN = import.meta.env.VITE_MOONLIGHT_ADMIN_TOKEN;
export const usesDemoData = import.meta.env.VITE_MOONLIGHT_DEMO === "true";

async function getJson<T>(path: string): Promise<T> {
  const headers = ADMIN_TOKEN ? { Authorization: `Bearer ${ADMIN_TOKEN}` } : undefined;
  const response = await fetch(`${API_BASE}${path}`, { headers });
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

async function putJson<T>(path: string, body: unknown): Promise<T> {
  const headers = {
    "content-type": "application/json",
    ...(ADMIN_TOKEN ? { Authorization: `Bearer ${ADMIN_TOKEN}` } : {})
  };
  const response = await fetch(`${API_BASE}${path}`, {
    method: "PUT",
    headers,
    body: JSON.stringify(body)
  });
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export type RunFilters = {
  classification?: Classification | "";
  adapter?: Adapter | "";
  q?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

function runsPath(filters: RunFilters = {}) {
  const params = new URLSearchParams();
  params.set("limit", String(filters.limit ?? 100));
  if (filters.offset) params.set("offset", String(filters.offset));
  if (filters.classification) params.set("classification", filters.classification);
  if (filters.adapter) params.set("adapter", filters.adapter);
  if (filters.q?.trim()) params.set("q", filters.q.trim());
  if (filters.status?.trim()) params.set("status", filters.status.trim());
  const query = params.toString();
  return `/api/runs${query ? `?${query}` : ""}`;
}

function filterDemoRuns(filters: RunFilters = {}): RunPage {
  const limit = filters.limit ?? 100;
  const offset = filters.offset ?? 0;
  const filtered = demoRunList.filter((run) => {
    if (filters.classification && run.classification !== filters.classification) return false;
    if (filters.adapter && run.adapter !== filters.adapter) return false;
    if (filters.status?.trim()) {
      const status = Number(filters.status);
      if ([run.primary_status, run.candidate_status, run.secondary_status].every((value) => value !== status)) {
        return false;
      }
    }
    if (filters.q?.trim()) {
      const query = filters.q.trim().toLowerCase();
      if (!JSON.stringify(run.input).toLowerCase().includes(query)) return false;
    }
    return true;
  });
  const items = filtered.slice(offset, offset + limit);
  const nextOffset = offset + items.length < filtered.length ? offset + items.length : null;
  return {
    items,
    limit,
    offset,
    total: filtered.length,
    next_offset: nextOffset
  };
}

export const api = {
  config: () => usesDemoData ? Promise.resolve(demoConfig) : getJson<AppConfig>("/api/config"),
  runs: (filters?: RunFilters) => usesDemoData ? Promise.resolve(filterDemoRuns(filters)) : getJson<RunPage>(runsPath(filters)),
  run: (id: string) => {
    if (usesDemoData) {
      const run = demoRuns.find((candidate) => candidate.id === id);
      return run ? Promise.resolve(run) : Promise.reject(new Error(`Demo run ${id} not found`));
    }
    return getJson<ComparisonRun>(`/api/runs/${id}`);
  },
  stats: () => usesDemoData ? Promise.resolve(demoStats) : getJson<StatsSummary>("/api/stats"),
  review: (id: string) => {
    if (usesDemoData) {
      return Promise.resolve(demoReviewStates[id] ?? {
        run_id: id,
        status: "new" as const,
        note: null,
        tags: [],
        updated_at: new Date().toISOString()
      });
    }
    return getJson<RunReviewState>(`/api/runs/${id}/review`);
  },
  updateReview: (id: string, update: ReviewUpdate) => {
    if (usesDemoData) {
      return Promise.resolve({
        run_id: id,
        status: update.status,
        note: update.note ?? null,
        tags: update.tags ?? [],
        updated_at: new Date().toISOString()
      });
    }
    return putJson<RunReviewState>(`/api/runs/${id}/review`, update);
  },
  reportUrl: (id: string, format: "markdown" | "json") => `${API_BASE}/api/runs/${id}/report?format=${format}`
};
