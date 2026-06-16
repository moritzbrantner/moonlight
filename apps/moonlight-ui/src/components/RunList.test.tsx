import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { runListFixture } from "../test/fixtures";
import { RunList } from "./RunList";

describe("RunList", () => {
  it("renders count, rows, classifications, statuses, and selected state", () => {
    render(<RunList runs={runListFixture} selectedId={runListFixture[0].id} onSelect={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "Runs" })).toBeInTheDocument();
    expect(screen.getByText(String(runListFixture.length))).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /GET \/regression/i })).toHaveClass("selected");
    expect(screen.getByText("Suspicious Difference")).toBeInTheDocument();
    expect(screen.getAllByText("200 / 200").length).toBeGreaterThan(0);
  });

  it("calls onSelect with selected run id", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<RunList runs={runListFixture} selectedId={null} onSelect={onSelect} />);

    await user.click(screen.getByRole("button", { name: /GET \/noise/i }));
    expect(onSelect).toHaveBeenCalledWith("demo-noise");
  });

  it("emits filter changes and load more requests", async () => {
    const user = userEvent.setup();
    const onFiltersChange = vi.fn();
    const onLoadMore = vi.fn();
    render(
      <RunList
        runs={runListFixture.slice(0, 1)}
        runTotal={runListFixture.length}
        selectedId={null}
        onFiltersChange={onFiltersChange}
        onLoadMore={onLoadMore}
        onSelect={vi.fn()}
      />
    );

    await user.type(screen.getByLabelText("Search runs"), "regression");
    await user.selectOptions(screen.getByLabelText("Filter by adapter"), "http");
    await user.click(screen.getByRole("button", { name: "Load more" }));

    expect(onFiltersChange).toHaveBeenCalled();
    expect(onLoadMore).toHaveBeenCalled();
  });

  it("shows empty state", () => {
    render(<RunList runs={[]} selectedId={null} onSelect={vi.fn()} />);

    expect(screen.getByText("No runs recorded yet.")).toBeInTheDocument();
  });
});
