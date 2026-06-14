import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { runFixture, runListFixture } from "../test/fixtures";
import { RunDetail } from "./RunDetail";

describe("RunDetail", () => {
  it("shows placeholder when no run is selected", () => {
    render(<RunDetail run={null} fallback={null} />);

    expect(screen.getByRole("heading", { name: "Run detail" })).toBeInTheDocument();
    expect(screen.getByText("Select a run to inspect target observations and diffs.")).toBeInTheDocument();
  });

  it("uses fallback title without a full run", () => {
    render(<RunDetail run={null} fallback={runListFixture[0]} />);

    expect(screen.getByRole("heading", { name: "GET /regression" })).toBeInTheDocument();
  });

  it("renders target cards, timestamp, and diff viewers for a full run", () => {
    render(<RunDetail run={runFixture} fallback={runListFixture[0]} />);

    expect(screen.getByRole("heading", { name: "GET /regression" })).toBeInTheDocument();
    expect(screen.getByText(new Date(runFixture.timestamp).toLocaleString())).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Primary Reference" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Candidate" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Secondary Reference" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Noise-filtered diff" })).toBeInTheDocument();
    expect(screen.getAllByText("$.value").length).toBeGreaterThan(0);
  });
});
