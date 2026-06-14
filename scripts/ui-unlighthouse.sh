#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/performance-results/unlighthouse"
SUMMARY_PATH="$ROOT_DIR/performance-results/unlighthouse-summary.md"
PREVIEW_LOG="$ROOT_DIR/performance-results/unlighthouse-preview.log"
SITE_URL="http://127.0.0.1:4173"
URLS="/?page=overview,/"

mkdir -p "$ROOT_DIR/performance-results"
rm -rf "$OUTPUT_DIR"

VITE_MOONLIGHT_DEMO=true VITE_MOONLIGHT_BASE_PATH=/ bun run build

(
  cd "$ROOT_DIR/apps/moonlight-ui"
  bunx vite preview --host 127.0.0.1 --port 4173 >"$PREVIEW_LOG" 2>&1
) &
PREVIEW_PID=$!

cleanup() {
  if kill -0 "$PREVIEW_PID" >/dev/null 2>&1; then
    kill "$PREVIEW_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

for _ in $(seq 1 60); do
  if curl -fsS "$SITE_URL/?page=overview" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

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
  echo "# UI Unlighthouse Benchmark"
  echo
  echo "- Site: \`$SITE_URL\`"
  echo "- URLs: \`/?page=overview\`, \`/\`"
  echo "- Mode: static demo UI"
  echo "- Report: \`performance-results/unlighthouse\`"
  echo
  echo "Generated files:"
  while IFS= read -r file; do
    relative_path="${file#"$ROOT_DIR/"}"
    printf -- "- \`%s\`\n" "$relative_path"
  done < <(find "$OUTPUT_DIR" -maxdepth 2 -type f | sort)
} >"$SUMMARY_PATH"
