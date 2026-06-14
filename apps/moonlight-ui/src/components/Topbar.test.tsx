import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Topbar } from "./Topbar";

describe("Topbar", () => {
  it("renders brand, page buttons, and active page state", () => {
    render(<Topbar page="overview" onNavigate={vi.fn()} onRefresh={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Moonlight overview" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Overview" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Dashboard" })).not.toHaveAttribute("aria-current");
    expect(screen.queryByRole("button", { name: "Refresh data" })).not.toBeInTheDocument();
  });

  it("shows refresh on dashboard and calls handlers", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    render(<Topbar page="dashboard" onNavigate={onNavigate} onRefresh={onRefresh} />);

    expect(screen.getByRole("button", { name: "Dashboard" })).toHaveAttribute("aria-current", "page");
    await user.click(screen.getByRole("button", { name: "Refresh data" }));
    expect(onRefresh).toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Overview" }));
    expect(onNavigate).toHaveBeenCalledWith("overview");
  });
});
