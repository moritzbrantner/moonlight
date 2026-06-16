import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const outputPath = resolve("apps/moonlight-ui/src/generated/api-types.ts");

const result = spawnSync("cargo", ["run", "-q", "-p", "moonlight-core", "--bin", "export-api-types"], {
  cwd: resolve("."),
  encoding: "utf8"
});

if (result.error) {
  throw result.error;
}

if (result.status !== 0) {
  process.stderr.write(result.stderr);
  process.exit(result.status ?? 1);
}

const content = `${result.stdout.trimEnd()}\n`;

if (process.argv.includes("--check")) {
  const current = readFileSync(outputPath, "utf8");
  if (current !== content) {
    console.error(`${outputPath} is stale. Run: bun scripts/generate-ui-api-types.mjs`);
    process.exit(1);
  }
} else {
  writeFileSync(outputPath, content);
}
