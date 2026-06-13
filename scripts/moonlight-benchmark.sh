#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILES=(
  -f docker-compose.moonlight.yml
  -f docker-compose.moonlight-benchmark.yml
)

docker compose "${COMPOSE_FILES[@]}" --profile reference up \
  -d \
  --build \
  primary \
  candidate \
  secondary \
  moonlight-http \
  diffy-a \
  diffy-b \
  diffy-c

python3 scripts/moonlight-benchmark.py "$@"

echo "Markdown summary: data/moonlight/benchmark/latest.md"
