import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { runFixture, targetErrorFixture, targetWithStderrFixture } from "../test/fixtures";
import { TargetCard } from "./TargetCard";

describe("TargetCard", () => {
  it("renders status, latency, body metadata, and preview", () => {
    render(<TargetCard title="Candidate" target={runFixture.candidate} />);

    expect(screen.getByRole("heading", { name: "Candidate" })).toBeInTheDocument();
    expect(screen.getByText(String(runFixture.candidate.status))).toBeInTheDocument();
    expect(screen.getByText(`${runFixture.candidate.latency_ms} ms`)).toBeInTheDocument();
    expect(screen.getByText(`${runFixture.candidate.body.size_bytes} bytes`)).toBeInTheDocument();
    expect(screen.getByText(runFixture.candidate.body.preview)).toBeInTheDocument();
  });

  it("renders error and stderr states", () => {
    render(
      <>
        <TargetCard title="Error target" target={targetErrorFixture} />
        <TargetCard title="Stderr target" target={targetWithStderrFixture} />
      </>
    );

    expect(screen.getByText("connection refused")).toBeInTheDocument();
    expect(screen.getByText("(empty)")).toBeInTheDocument();
    expect(screen.getByText("warning text")).toBeInTheDocument();
  });

  it("renders disabled state for missing secondary target", () => {
    render(<TargetCard title="Secondary Reference" target={null} />);

    expect(screen.getByRole("heading", { name: "Secondary Reference" })).toBeInTheDocument();
    expect(screen.getByText("Disabled")).toBeInTheDocument();
  });
});
