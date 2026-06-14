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

  it("shows empty state", () => {
    render(<RunList runs={[]} selectedId={null} onSelect={vi.fn()} />);

    expect(screen.getByText("No runs recorded yet.")).toBeInTheDocument();
  });
});
