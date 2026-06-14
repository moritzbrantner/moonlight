import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LatencyCells } from "./LatencyCells";

describe("LatencyCells", () => {
  it("renders p50, p95, p99, mean, and max values", () => {
    render(
      <table>
        <tbody>
          <tr>
            <LatencyCells latency={{ min: 1, mean: 4.567, p50: 2.1, p90: 6, p95: 7.89, p99: 9.01, max: 12.3 }} />
          </tr>
        </tbody>
      </table>
    );

    expect(screen.getByText("2.10")).toBeInTheDocument();
    expect(screen.getByText("7.89")).toBeInTheDocument();
    expect(screen.getByText("9.01")).toBeInTheDocument();
    expect(screen.getByText("4.57")).toBeInTheDocument();
    expect(screen.getByText("12.30")).toBeInTheDocument();
  });

  it("renders dash for missing latency values", () => {
    render(
      <table>
        <tbody>
          <tr>
            <LatencyCells latency={{ min: null, mean: null, p50: null, p90: null, p95: null, p99: null, max: null }} />
          </tr>
        </tbody>
      </table>
    );

    expect(screen.getAllByText("-")).toHaveLength(5);
  });
});
