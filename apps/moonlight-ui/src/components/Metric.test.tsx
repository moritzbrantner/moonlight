import { Activity } from "lucide-react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Metric } from "./Metric";

describe("Metric", () => {
  it("renders label, value, and icon", () => {
    render(<Metric label="Total" value={42} icon={<Activity aria-label="activity" />} />);

    expect(screen.getByText("Total")).toBeInTheDocument();
    expect(screen.getByText("42")).toBeInTheDocument();
    expect(screen.getByLabelText("activity")).toBeInTheDocument();
  });
});
