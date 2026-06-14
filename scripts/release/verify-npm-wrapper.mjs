#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const wrapper = "packages/npm/moonlight/bin/moonlight.js";
const realBinary = process.env.MOONLIGHT_BIN ?? process.argv[2];

function runWrapper(args, extraEnv = {}) {
  return spawnSync(process.execPath, [wrapper, ...args], {
    env: { ...process.env, ...extraEnv },
    encoding: "utf8",
    stdio: "pipe",
  });
}

function expectStatus(label, result, expectedStatus) {
  if (result.error) {
    throw new Error(`${label} failed to spawn: ${result.error.message}`);
  }
  if (result.status !== expectedStatus) {
    throw new Error(
      `${label} exited with ${result.status}; expected ${expectedStatus}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
    );
  }
}

function expectOutput(label, output, expected) {
  if (!output.includes(expected)) {
    throw new Error(`${label} did not include ${expected}\n${output}`);
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
    ? "@echo off\r\necho fake-wrapper-override %*\r\nexit /b 42\r\n"
    : "#!/usr/bin/env sh\necho fake-wrapper-override \"$@\"\nexit 42\n";

writeFileSync(fakeBinary, fakeScript);
await chmod(fakeBinary, 0o755);

try {
  expectStatus(
    "wrapper exit propagation",
    runWrapper(["ignored"], { MOONLIGHT_BIN: fakeBinary }),
    42,
  );
  const override = runWrapper(["arg-one"], { MOONLIGHT_BIN: fakeBinary });
  expectStatus("MOONLIGHT_BIN override precedence", override, 42);
  expectOutput("MOONLIGHT_BIN override precedence", override.stdout, "fake-wrapper-override");
  expectOutput("MOONLIGHT_BIN argument forwarding", override.stdout, "arg-one");
} finally {
  await rm(tempDir, { force: true, recursive: true });
}

const wrapperSource = readFileSync(wrapper, "utf8");
expectOutput(
  "missing optional package guidance",
  wrapperSource,
  "optional native package",
);
expectOutput(
  "missing optional package remediation",
  wrapperSource,
  "optional dependencies enabled",
);
expectOutput("unsupported platform guidance", wrapperSource, "cargo install moonlight-cli --locked");

console.log("npm wrapper smoke checks passed");
