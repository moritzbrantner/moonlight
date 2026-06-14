#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const require = createRequire(import.meta.url);

const platformPackages = {
  "darwin-arm64": {
    packageName: "@moritzbrantner/moonlight-darwin-arm64",
    binary: "bin/moonlight",
  },
  "darwin-x64": {
    packageName: "@moritzbrantner/moonlight-darwin-x64",
    binary: "bin/moonlight",
  },
  "linux-arm64": {
    packageName: "@moritzbrantner/moonlight-linux-arm64-gnu",
    binary: "bin/moonlight",
  },
  "linux-x64": {
    packageName: "@moritzbrantner/moonlight-linux-x64-gnu",
    binary: "bin/moonlight",
  },
  "win32-x64": {
    packageName: "@moritzbrantner/moonlight-win32-x64-msvc",
    binary: "bin/moonlight.exe",
  },
};

function resolveBinary() {
  if (process.env.MOONLIGHT_BIN) {
    return process.env.MOONLIGHT_BIN;
  }

  const platformKey = `${process.platform}-${process.arch}`;
  const platformPackage = platformPackages[platformKey];
  if (!platformPackage) {
    throw new Error(
      `Moonlight does not publish an npm binary for ${process.platform}/${process.arch}. ` +
        "Install with `cargo install moonlight-cli --locked` or set MOONLIGHT_BIN.",
    );
  }

  let packageJsonPath;
  try {
    packageJsonPath = require.resolve(`${platformPackage.packageName}/package.json`);
  } catch {
    throw new Error(
      `The optional native package ${platformPackage.packageName} is not installed. ` +
        "Reinstall @moritzbrantner/moonlight with optional dependencies enabled, " +
        "or set MOONLIGHT_BIN to a local moonlight executable.",
    );
  }

  const binaryPath = join(dirname(packageJsonPath), platformPackage.binary);
  if (!existsSync(binaryPath)) {
    throw new Error(`The native Moonlight binary is missing at ${binaryPath}.`);
  }

  return binaryPath;
}

let binary;
try {
  binary = resolveBinary();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(`Failed to run ${binary}: ${result.error.message}`);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
