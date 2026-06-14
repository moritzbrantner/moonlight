import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BenchmarkSection } from "./BenchmarkSection";

describe("BenchmarkSection", () => {
  it("renders heading, generated timestamp, details, and children", () => {
    render(
      <BenchmarkSection title="HTTP Benchmark" generatedAt="2026-06-13T12:00:00.000Z" details="600 requests">
        <p>benchmark rows</p>
      </BenchmarkSection>
    );

    expect(screen.getByRole("heading", { name: "HTTP Benchmark" })).toBeInTheDocument();
    expect(screen.getByText("600 requests")).toBeInTheDocument();
    expect(screen.getByText("benchmark rows")).toBeInTheDocument();
    expect(screen.getByText(new Date("2026-06-13T12:00:00.000Z").toLocaleString())).toHaveAttribute(
      "dateTime",
      "2026-06-13T12:00:00.000Z"
    );
  });
});
