#!/usr/bin/env bash
set -euo pipefail

cargo build --release -p moonlight-cli
python3 scripts/moonlight-cli-benchmark.py "$@"

echo "moonlight CLI benchmark complete"
