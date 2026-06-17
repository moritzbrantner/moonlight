#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CASES="${MOONLIGHT_SELFDOGFOOD_CASES:-$ROOT/tests/selfdogfood/cases.jsonl}"
WORK_DIR="${MOONLIGHT_SELFDOGFOOD_OUT:-$ROOT/.moonlight/selfdogfood/$(date -u +%Y%m%dT%H%M%SZ)}"

cd "$ROOT"

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
elif command -v npx >/dev/null 2>&1; then
  REFERENCE=(npx -y @moritzbrantner/moonlight@latest)
else
  cat >&2 <<'EOF'
Could not resolve a published Moonlight CLI binary.
Install npm/npx or set MOONLIGHT_PUBLISHED_BIN to an executable stable Moonlight binary, for example:

  MOONLIGHT_PUBLISHED_BIN=/path/to/stable/moonlight tests/selfdogfood/run-published-vs-source.sh
EOF
  exit 1
fi

python3 "$ROOT/tests/selfdogfood/normalize-output.py" \
  --cases "$CASES" \
  --work-dir "$WORK_DIR" \
  --candidate "$CANDIDATE" \
  --reference "${REFERENCE[@]}"
