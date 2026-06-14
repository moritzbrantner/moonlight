import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { DiffEntry } from "../types";
import { runFixture } from "../test/fixtures";
import { DiffViewer } from "./DiffViewer";

describe("DiffViewer", () => {
  it("renders count and empty state", () => {
    render(<DiffViewer title="Raw candidate diff" diffs={[]} />);

    expect(screen.getByRole("heading", { name: "Raw candidate diff" })).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();
    expect(screen.getByText("No entries.")).toBeInTheDocument();
  });

  it("renders diff fields and replaces missing values with dashes", () => {
    const diff: DiffEntry = {
      ...runFixture.comparison.raw_candidate_diffs[0],
      candidate: null,
      secondary: null
    };

    render(<DiffViewer title="Noise-filtered diff" diffs={[diff]} />);

    expect(screen.getByText("body")).toBeInTheDocument();
    expect(screen.getByText("$.value")).toBeInTheDocument();
    expect(screen.getByText("P: 42")).toBeInTheDocument();
    expect(screen.getByText("C: -")).toBeInTheDocument();
    expect(screen.getByText("S: -")).toBeInTheDocument();
  });
});
