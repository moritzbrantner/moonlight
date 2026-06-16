import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { configFixture } from "../test/fixtures";
import { ConfigPanel } from "./ConfigPanel";

describe("ConfigPanel", () => {
  it("renders configured targets and redaction settings", () => {
    render(<ConfigPanel config={configFixture} />);

    expect(screen.getByRole("heading", { name: "Config" })).toBeInTheDocument();
    expect(screen.getByText(configFixture.primary_url)).toBeInTheDocument();
    expect(screen.getByText(configFixture.candidate_url)).toBeInTheDocument();
    expect(screen.getByText(configFixture.secondary_url)).toBeInTheDocument();
    expect(screen.getByText(configFixture.redact_query_params.join(", "))).toBeInTheDocument();
    expect(screen.getByText(configFixture.ignore_json_paths.join(", "))).toBeInTheDocument();
  });

  it("renders unavailable state without config", () => {
    render(<ConfigPanel config={null} />);

    expect(screen.getByText("Unavailable")).toBeInTheDocument();
  });
});
