import type { Classification, RunInput } from "../types";

export function runTitle(input: RunInput) {
  if ("method" in input) {
    return `${input.method} ${input.path}${input.query ? `?${input.query}` : ""}`;
  }
  if ("project" in input) {
    return `${input.project} / ${input.check_id}`;
  }
  return input.candidate_command;
}

export function labelFor(classification: Classification) {
  return classification
    .split("_")
    .map((word) => word[0].toUpperCase() + word.slice(1))
    .join(" ");
}
