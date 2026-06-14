import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { configFixture, runFixture, runListFixture, statsFixture } from "../test/fixtures";

vi.mock("../api", () => ({ usesDemoData: true }));

import { DashboardPage } from "./DashboardPage";

describe("DashboardPage", () => {
  it("renders metrics, selected run detail, and side panels", () => {
    render(
      <DashboardPage
        config={configFixture}
        error={null}
        loading={false}
        onSelectRun={vi.fn()}
        runs={runListFixture}
        selected={runFixture}
        selectedFromList={runListFixture[0]}
        selectedId={runFixture.id}
        stats={statsFixture}
      />
    );

    expect(screen.getByText("Total")).toBeInTheDocument();
    expect(screen.getAllByText(String(statsFixture.total_runs)).length).toBeGreaterThan(0);
    expect(screen.getByText("Suspicious")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Runs" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Config" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Latency" })).toBeInTheDocument();
    expect(screen.getAllByText("GET /regression").length).toBeGreaterThan(0);
  });

  it("shows error, demo data, loading, and empty run states", () => {
    render(
      <DashboardPage
        config={null}
        error="API failed"
        loading={true}
        onSelectRun={vi.fn()}
        runs={[]}
        selected={null}
        selectedFromList={null}
        selectedId={null}
        stats={null}
      />
    );

    expect(screen.getByText("API failed")).toBeInTheDocument();
    expect(screen.getByText("Demo data for the GitHub Pages example.")).toBeInTheDocument();
    expect(screen.getByText("Loading admin API data...")).toBeInTheDocument();
    expect(screen.getByText("No runs recorded yet.")).toBeInTheDocument();
    expect(screen.getByText("Select a run to inspect target observations and diffs.")).toBeInTheDocument();
  });
});
