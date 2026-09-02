import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDirectory, "..");
const repositoryRoot = resolve(appRoot, "../..");
const publicRoot = join(appRoot, "public");

const agentTool = JSON.parse(await readFile(join(publicRoot, "agent-tool.json"), "utf8"));
const reportIndex = JSON.parse(await readFile(join(publicRoot, "reports/index.json"), "utf8"));

assert.equal(agentTool.schemaVersion, 1);
assert.equal(agentTool.id, "moonlight");
assert.equal(agentTool.kind, "evaluation-report-catalog");
assert.ok(agentTool.operations.some((operation) => operation.id === "reports"));
assert.ok(Array.isArray(agentTool.authoritativeLocalOperations));
assert.ok(agentTool.authoritativeLocalOperations.length > 0);

assert.equal(reportIndex.schemaVersion, 1);
assert.equal(reportIndex.tool, "moonlight");
assert.deepEqual(
  reportIndex.reports.map((report) => report.id).toSorted(),
  ["cli-benchmark-latest", "http-benchmark-latest"],
);

for (const [source, published] of [
  ["data/moonlight/benchmark/latest.json", "reports/http-latest.json"],
  ["data/moonlight/cli-benchmark-analysis/latest.json", "reports/cli-latest.json"],
]) {
  const sourceValue = JSON.parse(await readFile(join(repositoryRoot, source), "utf8"));
  const publishedValue = JSON.parse(await readFile(join(publicRoot, published), "utf8"));
  assert.deepEqual(publishedValue, sourceValue);
}

console.log("Moonlight agent Pages contract is valid.");
