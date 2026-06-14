#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const wrapper = "packages/npm/moonlight/bin/moonlight.js";
const realBinary = process.env.MOONLIGHT_BIN ?? process.argv[2];

function runWrapper(args, extraEnv = {}) {
  return spawnSync(process.execPath, [wrapper, ...args], {
    env: { ...process.env, ...extraEnv },
    stdio: "inherit",
  });
}

function expectStatus(label, result, expectedStatus) {
  if (result.error) {
    throw new Error(`${label} failed to spawn: ${result.error.message}`);
  }
  if (result.status !== expectedStatus) {
    throw new Error(`${label} exited with ${result.status}; expected ${expectedStatus}`);
  }
}

if (realBinary) {
  expectStatus(
    "real binary help",
    runWrapper(["--help"], { MOONLIGHT_BIN: realBinary }),
    0,
  );
}

const tempDir = mkdtempSync(join(tmpdir(), "moonlight-wrapper-"));
const fakeBinary = join(tempDir, process.platform === "win32" ? "moonlight.cmd" : "moonlight");
const fakeScript =
  process.platform === "win32"
    ? "@echo off\r\nexit /b 42\r\n"
    : "#!/usr/bin/env sh\nexit 42\n";

writeFileSync(fakeBinary, fakeScript);
await chmod(fakeBinary, 0o755);

try {
  expectStatus(
    "wrapper exit propagation",
    runWrapper(["ignored"], { MOONLIGHT_BIN: fakeBinary }),
    42,
  );
} finally {
  await rm(tempDir, { force: true, recursive: true });
}

console.log("npm wrapper smoke checks passed");
