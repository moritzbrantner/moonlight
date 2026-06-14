#!/usr/bin/env node
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const expectedVersion = process.argv[2]?.replace(/^v/, "");
const cargoToml = readFileSync("Cargo.toml", "utf8");
const workspaceVersion = cargoToml.match(
  /\[workspace\.package\][\s\S]*?^version = "([^"]+)"/m,
)?.[1];

if (!workspaceVersion) {
  throw new Error("Could not find [workspace.package] version in Cargo.toml");
}

if (expectedVersion && workspaceVersion !== expectedVersion) {
  throw new Error(
    `Cargo workspace version ${workspaceVersion} does not match expected ${expectedVersion}`,
  );
}

const packageFiles = [
  "packages/npm/moonlight/package.json",
  ...readdirSync("packages/npm/platforms", { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => join("packages/npm/platforms", entry.name, "package.json")),
];

const packageVersions = new Map();
for (const packageFile of packageFiles) {
  const packageJson = JSON.parse(readFileSync(packageFile, "utf8"));
  packageVersions.set(packageJson.name, packageJson.version);
  if (packageJson.version !== workspaceVersion) {
    throw new Error(
      `${packageFile} version ${packageJson.version} does not match ${workspaceVersion}`,
    );
  }
}

const mainPackage = JSON.parse(readFileSync("packages/npm/moonlight/package.json", "utf8"));
for (const [dependencyName, dependencyVersion] of Object.entries(
  mainPackage.optionalDependencies ?? {},
)) {
  const packageVersion = packageVersions.get(dependencyName);
  if (!packageVersion) {
    throw new Error(`Optional dependency ${dependencyName} has no platform package`);
  }
  if (dependencyVersion !== packageVersion) {
    throw new Error(
      `Optional dependency ${dependencyName}@${dependencyVersion} does not match package version ${packageVersion}`,
    );
  }
}

console.log(`release versions are consistent at ${workspaceVersion}`);
