#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/performance-results/unlighthouse-live"
SUMMARY_PATH="$ROOT_DIR/performance-results/unlighthouse-live-summary.md"
SITE_URL="${MOONLIGHT_UI_URL:-http://127.0.0.1:5173}"
API_URL="${VITE_MOONLIGHT_API_URL:-}"
URLS="/?page=overview,/"

mkdir -p "$ROOT_DIR/performance-results"
rm -rf "$OUTPUT_DIR"

curl -fsS "$SITE_URL/?page=overview" >/dev/null

bunx unlighthouse-ci \
  --site "$SITE_URL" \
  --urls "$URLS" \
  --output-path "$OUTPUT_DIR" \
  --reporter jsonExpanded \
  --build-static true \
  --desktop \
  --enable-javascript \
  --disable-robots-txt \
  --disable-sitemap \
  --disable-dynamic-sampling

{
  echo "# Live UI Unlighthouse Benchmark"
  echo
  echo "- Site: \`$SITE_URL\`"
  if [ -n "$API_URL" ]; then
    echo "- API: \`$API_URL\`"
  fi
  echo "- URLs: \`/?page=overview\`, \`/\`"
  echo "- Mode: live UI/API"
  echo "- Report: \`performance-results/unlighthouse-live\`"
  echo
  echo "Generated files:"
  while IFS= read -r file; do
    relative_path="${file#"$ROOT_DIR/"}"
    printf -- "- \`%s\`\n" "$relative_path"
  done < <(find "$OUTPUT_DIR" -maxdepth 2 -type f | sort)
} >"$SUMMARY_PATH"
