import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDirectory, "..");
const repositoryRoot = resolve(appRoot, "../..");
const publicRoot = join(appRoot, "public");
const reportsRoot = join(publicRoot, "reports");
const pagesBaseUrl = "https://moritzbrantner.github.io/moonlight/";

const reportSources = [
  {
    id: "http-benchmark-latest",
    source: join(repositoryRoot, "data/moonlight/benchmark/latest.json"),
    destination: "http-latest.json",
    description: "Latest committed Moonlight HTTP benchmark report.",
  },
  {
    id: "cli-benchmark-latest",
    source: join(repositoryRoot, "data/moonlight/cli-benchmark-analysis/latest.json"),
    destination: "cli-latest.json",
    description: "Latest committed Moonlight CLI benchmark analysis.",
  },
];

await mkdir(reportsRoot, { recursive: true });

const reports = [];
for (const report of reportSources) {
  const sourceText = await readFile(report.source, "utf8");
  JSON.parse(sourceText);
  await writeFile(join(reportsRoot, report.destination), sourceText.endsWith("\n") ? sourceText : `${sourceText}\n`);
  reports.push({
    id: report.id,
    href: `${pagesBaseUrl}reports/${report.destination}`,
    description: report.description,
  });
}

const reportIndex = {
  schemaVersion: 1,
  tool: "moonlight",
  reports,
};

const agentTool = {
  schemaVersion: 1,
  id: "moonlight",
  kind: "evaluation-report-catalog",
  baseUrl: pagesBaseUrl,
  description:
    "Inspect Moonlight's committed evaluation and benchmark evidence from GitHub Pages; execute comparisons locally with the Moonlight CLI.",
  operations: [
    {
      id: "reports",
      transport: "static-json",
      href: `${pagesBaseUrl}reports/index.json`,
      description: "Discover the latest committed benchmark/evaluation reports exposed on Pages.",
    },
    ...reports.map((report) => ({
      id: report.id,
      transport: "static-json",
      href: report.href,
      description: report.description,
    })),
    {
      id: "reference-ui",
      transport: "html",
      href: `${pagesBaseUrl}?page=overview`,
      description: "Human-readable Moonlight overview and benchmark report presentation.",
    },
  ],
  authoritativeLocalOperations: [
    "moonlight run --primary <command> --candidate <command>",
    "moonlight eval run --project moonlight.eval.toml --candidate-patch <patch> --format json",
    "moonlight eval report --storage-path <path> --format json",
  ],
  limitations: [
    "GitHub Pages exposes committed evidence and documentation only; it does not execute baseline/candidate workloads.",
    "Moonlight CLI evaluation remains authoritative for new comparisons and project checks.",
  ],
};

await writeFile(join(reportsRoot, "index.json"), `${JSON.stringify(reportIndex, null, 2)}\n`);
await writeFile(join(publicRoot, "agent-tool.json"), `${JSON.stringify(agentTool, null, 2)}\n`);

console.log(`Published ${reports.length} Moonlight reports for coding agents.`);
