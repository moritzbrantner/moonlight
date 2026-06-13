#!/usr/bin/env bash
set -euo pipefail

SHADOWDIFF_URL="${SHADOWDIFF_URL:-http://127.0.0.1:8080}"
DIFFY_URL="${DIFFY_URL:-http://127.0.0.1:8880}"
INCLUDE_DIFFY="${INCLUDE_DIFFY:-0}"
ROUNDS="${ROUNDS:-3}"

endpoints=(
  "/success"
  "/regression"
  "/noise"
  "/noisy-regression"
  "/status-regression"
  "/slow-candidate"
)

send_one() {
  local base_url="$1"
  local endpoint="$2"
  curl --silent --show-error --output /dev/null \
    --write-out "%{http_code} %{time_total}s ${base_url}${endpoint}\n" \
    "${base_url}${endpoint}"
}

for round in $(seq 1 "${ROUNDS}"); do
  echo "round ${round}/${ROUNDS}"
  for endpoint in "${endpoints[@]}"; do
    send_one "${SHADOWDIFF_URL}" "${endpoint}"
    if [[ "${INCLUDE_DIFFY}" == "1" ]]; then
      send_one "${DIFFY_URL}" "${endpoint}" || true
    fi
  done
done
