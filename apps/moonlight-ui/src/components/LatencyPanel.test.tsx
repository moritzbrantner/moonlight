import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { runFixture } from "../test/fixtures";
import { LatencyPanel } from "./LatencyPanel";

describe("LatencyPanel", () => {
  it("renders empty state when no run is selected", () => {
    render(<LatencyPanel run={null} />);

    expect(screen.getByRole("heading", { name: "Latency" })).toBeInTheDocument();
    expect(screen.getByText("Select a run.")).toBeInTheDocument();
  });

  it("renders latency metrics for selected run", () => {
    render(<LatencyPanel run={runFixture} />);

    expect(screen.getByText(`${runFixture.primary.latency_ms} ms`)).toBeInTheDocument();
    expect(screen.getByText(`${runFixture.candidate.latency_ms} ms`)).toBeInTheDocument();
    expect(screen.getByText(`${runFixture.secondary?.latency_ms} ms`)).toBeInTheDocument();
  });
});
