import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { httpBenchmarkTargetFixture } from "../test/fixtures";
import { formatNumber } from "../utils/format";
import { HttpBenchmarkRow } from "./HttpBenchmarkRow";

describe("HttpBenchmarkRow", () => {
  it("renders request counts, throughput, and latency cells", () => {
    render(
      <table>
        <tbody>
          <HttpBenchmarkRow target={httpBenchmarkTargetFixture} />
        </tbody>
      </table>
    );

    const row = screen.getByRole("row");
    expect(within(row).getByText(httpBenchmarkTargetFixture.name)).toBeInTheDocument();
    expect(within(row).getAllByText(String(httpBenchmarkTargetFixture.total_requests)).length).toBeGreaterThan(0);
    expect(within(row).getAllByText(String(httpBenchmarkTargetFixture.success_count)).length).toBeGreaterThan(0);
    expect(within(row).getByText(String(httpBenchmarkTargetFixture.error_count))).toBeInTheDocument();
    expect(within(row).getByText(formatNumber(httpBenchmarkTargetFixture.requests_per_second))).toBeInTheDocument();
  });
});
