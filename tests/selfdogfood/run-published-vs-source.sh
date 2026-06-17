#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CASES="${MOONLIGHT_SELFDOGFOOD_CASES:-$ROOT/tests/selfdogfood/cases.jsonl}"
WORK_DIR="${MOONLIGHT_SELFDOGFOOD_OUT:-$ROOT/.moonlight/selfdogfood/$(date -u +%Y%m%dT%H%M%SZ)}"

cd "$ROOT"
mkdir -p "$WORK_DIR"

cargo build --release -p moonlight-cli
CANDIDATE="$ROOT/target/release/moonlight"
if [[ ! -x "$CANDIDATE" ]]; then
  echo "Moonlight source build did not produce an executable at $CANDIDATE" >&2
  exit 1
fi

if [[ -n "${MOONLIGHT_PUBLISHED_BIN:-}" ]]; then
  if [[ ! -x "$MOONLIGHT_PUBLISHED_BIN" ]]; then
    echo "MOONLIGHT_PUBLISHED_BIN is set but is not executable: $MOONLIGHT_PUBLISHED_BIN" >&2
    exit 1
  fi
  REFERENCE=("$MOONLIGHT_PUBLISHED_BIN")
elif command -v npm >/dev/null 2>&1; then
  RESOLVED_REFERENCE="$WORK_DIR/published-moonlight.path"
  if ! npm exec --yes --package @moritzbrantner/moonlight@latest -- node -e '
const { spawnSync } = require("node:child_process");
const command = process.platform === "win32" ? "where" : "which";
const result = spawnSync(command, ["moonlight"], { encoding: "utf8" });
if (result.status !== 0) process.exit(result.status ?? 1);
const path = result.stdout.split(/\r?\n/).find(Boolean);
if (!path) process.exit(1);
process.stdout.write(path);
' >"$RESOLVED_REFERENCE"; then
    cat >&2 <<'EOF'
Could not resolve a published Moonlight CLI binary via npm.
Set MOONLIGHT_PUBLISHED_BIN to an executable stable Moonlight binary, for example:

  MOONLIGHT_PUBLISHED_BIN=/path/to/stable/moonlight tests/selfdogfood/run-published-vs-source.sh
EOF
    exit 1
  fi
  REFERENCE_BIN="$(cat "$RESOLVED_REFERENCE")"
  if [[ ! -x "$REFERENCE_BIN" ]]; then
    echo "Resolved published Moonlight CLI is not executable: $REFERENCE_BIN" >&2
    echo "Set MOONLIGHT_PUBLISHED_BIN to an executable stable Moonlight binary." >&2
    exit 1
  fi
  if ! "$REFERENCE_BIN" --help >/dev/null 2>"$WORK_DIR/published-moonlight.validation.stderr"; then
    echo "Resolved published Moonlight CLI failed validation: $REFERENCE_BIN" >&2
    echo "See $WORK_DIR/published-moonlight.validation.stderr" >&2
    exit 1
  fi
  REFERENCE=("$REFERENCE_BIN")
else
  cat >&2 <<'EOF'
Could not resolve a published Moonlight CLI binary.
Install npm or set MOONLIGHT_PUBLISHED_BIN to an executable stable Moonlight binary, for example:

  MOONLIGHT_PUBLISHED_BIN=/path/to/stable/moonlight tests/selfdogfood/run-published-vs-source.sh
EOF
  exit 1
fi

python3 "$ROOT/tests/selfdogfood/normalize-output.py" \
  --cases "$CASES" \
  --work-dir "$WORK_DIR" \
  --candidate "$CANDIDATE" \
  --reference "${REFERENCE[@]}"
