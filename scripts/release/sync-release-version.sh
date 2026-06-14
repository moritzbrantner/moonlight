#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>" >&2
  exit 2
fi

version="${1#v}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]]; then
  echo "invalid SemVer version: $1" >&2
  exit 2
fi

VERSION="$version" perl -0pi -e 's/(\[workspace\.package\][\s\S]*?^version = ")[^"]+(")/$1$ENV{VERSION}$2/m' Cargo.toml

node - "$version" <<'JS'
const fs = require("node:fs");
const path = require("node:path");

const version = process.argv[2];
const packageFiles = [
  "packages/npm/moonlight/package.json",
  ...fs
    .readdirSync("packages/npm/platforms", { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join("packages/npm/platforms", entry.name, "package.json")),
];

for (const packageFile of packageFiles) {
  const packageJson = JSON.parse(fs.readFileSync(packageFile, "utf8"));
  packageJson.version = version;

  if (packageJson.name === "@moritzbrantner/moonlight") {
    for (const dependencyName of Object.keys(packageJson.optionalDependencies ?? {})) {
      packageJson.optionalDependencies[dependencyName] = version;
    }
  }

  fs.writeFileSync(packageFile, `${JSON.stringify(packageJson, null, 2)}\n`);
}
JS

echo "synced release version $version"
