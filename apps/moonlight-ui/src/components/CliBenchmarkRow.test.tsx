import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { cliBenchmarkComparisonFixture, skippedCliComparisonFixture } from "../test/fixtures";
import { CliBenchmarkRow } from "./CliBenchmarkRow";

describe("CliBenchmarkRow", () => {
  it("renders ok comparison totals and normalized target latency", () => {
    render(
      <table>
        <tbody>
          <CliBenchmarkRow name="moonlight" comparison={cliBenchmarkComparisonFixture} />
        </tbody>
      </table>
    );

    const row = screen.getByRole("row");
    expect(within(row).getByText("moonlight")).toBeInTheDocument();
    expect(within(row).getByText(cliBenchmarkComparisonFixture.status)).toBeInTheDocument();
    expect(within(row).getByText(String(cliBenchmarkComparisonFixture.total_cases))).toBeInTheDocument();
    expect(within(row).getByText(String(cliBenchmarkComparisonFixture.total_target_invocations))).toBeInTheDocument();
  });

  it("renders skipped comparison reason and empty latency values", () => {
    render(
      <table>
        <tbody>
          <CliBenchmarkRow name="bats" comparison={skippedCliComparisonFixture} />
        </tbody>
      </table>
    );

    expect(screen.getByText("skipped")).toBeInTheDocument();
    expect(screen.getByText("tool unavailable")).toBeInTheDocument();
    expect(screen.getAllByText("-").length).toBeGreaterThan(0);
  });
});
