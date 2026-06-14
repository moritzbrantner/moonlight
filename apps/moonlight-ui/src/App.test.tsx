import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { configFixture, runFixture, runListFixture, statsFixture } from "./test/fixtures";

async function renderApp(url = "/") {
  window.history.replaceState({}, "", url);
  const api = {
    config: vi.fn().mockResolvedValue(configFixture),
    runs: vi.fn().mockResolvedValue(runListFixture),
    run: vi.fn().mockResolvedValue(runFixture),
    stats: vi.fn().mockResolvedValue(statsFixture)
  };
  vi.doMock("./api", () => ({ api, usesDemoData: true }));
  const { App } = await import("./App");
  render(<App />);
  return api;
}

describe("App", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllTimers();
    vi.resetModules();
    vi.restoreAllMocks();
  });

  it("defaults to the dashboard without an overview query param", async () => {
    await renderApp("/");

    expect(await screen.findByRole("heading", { name: "Runs" })).toBeInTheDocument();
    expect(await screen.findByText("Demo data for the GitHub Pages example.")).toBeInTheDocument();
  });

  it("renders overview when URL has page=overview", async () => {
    await renderApp("/?page=overview");

    expect(screen.getByRole("heading", { name: "Reference and candidate checks for HTTP and CLI targets." })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Runs" })).not.toBeInTheDocument();
  });

  it("updates page and URL when navigating", async () => {
    const user = userEvent.setup();
    await renderApp("/");

    await user.click(screen.getByRole("button", { name: "Overview" }));
    expect(screen.getByRole("heading", { name: "Reference and candidate checks for HTTP and CLI targets." })).toBeInTheDocument();
    expect(window.location.search).toBe("?page=overview");

    await user.click(screen.getByRole("button", { name: "Dashboard" }));
    expect(await screen.findByRole("heading", { name: "Runs" })).toBeInTheDocument();
    expect(window.location.search).toBe("");
  });

  it("loads demo dashboard data and refreshes without crashing", async () => {
    const user = userEvent.setup();
    const api = await renderApp("/");

    expect((await screen.findAllByText("GET /regression")).length).toBeGreaterThan(0);
    expect(screen.getByText("Primary avg")).toBeInTheDocument();
    expect(api.stats).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "Refresh data" }));
    await waitFor(() => expect(api.stats).toHaveBeenCalledTimes(2));
  });
});
