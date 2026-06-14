import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { cliBenchmark, httpBenchmark } from "../benchmarkData";
import { OverviewPage } from "./OverviewPage";

describe("OverviewPage", () => {
  it("renders hero, repository content, and benchmark summary", () => {
    render(<OverviewPage onNavigate={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "Reference and candidate checks for HTTP and CLI targets." })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /repository/i })).toHaveAttribute("href", "https://github.com/moritzbrantner/moonlight");
    expect(screen.getByLabelText("Latest benchmark summary")).toBeInTheDocument();
    expect(screen.getByText("moonlight-core")).toBeInTheDocument();
  });

  it("renders HTTP and CLI benchmark rows", () => {
    render(<OverviewPage onNavigate={vi.fn()} />);

    for (const target of ["moonlight", "diffy_b", "diffy_c"]) {
      expect(screen.getAllByText(httpBenchmark.targets[target].name).length).toBeGreaterThan(0);
    }

    for (const target of cliBenchmark.config.targets) {
      expect(screen.getAllByText(target).length).toBeGreaterThan(0);
    }
  });

  it("navigates to dashboard from hero action", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();
    render(<OverviewPage onNavigate={onNavigate} />);

    await user.click(screen.getByRole("button", { name: /demo dashboard/i }));
    expect(onNavigate).toHaveBeenCalledWith("dashboard");
  });
});
