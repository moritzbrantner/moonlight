import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
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

  it("renders report links and review actions", async () => {
    const user = userEvent.setup();
    const onUpdateReview = vi.fn();
    render(
      <RunDetail
        run={runFixture}
        fallback={runListFixture[0]}
        review={{
          run_id: runFixture.id,
          status: "new",
          note: null,
          tags: [],
          updated_at: runFixture.timestamp
        }}
        onUpdateReview={onUpdateReview}
      />
    );

    expect(screen.getByRole("link", { name: "Markdown" })).toHaveAttribute("href", `/api/runs/${runFixture.id}/report?format=markdown`);
    expect(screen.getByRole("link", { name: "JSON" })).toHaveAttribute("href", `/api/runs/${runFixture.id}/report?format=json`);
    await user.click(screen.getByRole("button", { name: "Ignore" }));
    expect(onUpdateReview).toHaveBeenCalledWith({ status: "ignored", note: null, tags: [] });
  });
});
