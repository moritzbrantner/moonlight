#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILES=(
  -f docker-compose.shadowdiff.yml
  -f docker-compose.shadowdiff-benchmark.yml
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

python3 scripts/shadowdiff-benchmark.py "$@"

echo "Markdown summary: data/shadowdiff/benchmark/latest.md"
