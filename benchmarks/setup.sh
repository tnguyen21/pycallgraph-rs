#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VENV_DIR="$REPO_ROOT/benchmarks/.venv"

"$REPO_ROOT/scripts/bootstrap-corpora.sh" --only-corpora

if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
fi

"$VENV_DIR/bin/pip" install --upgrade pip >/dev/null
"$VENV_DIR/bin/pip" install code2flow >/dev/null

echo "Benchmark environment ready."
echo "Use:"
echo "  python3 benchmarks/bench.py --pycg ./target/release/pycg"
