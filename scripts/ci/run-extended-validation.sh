#!/usr/bin/env bash
set -euo pipefail
bash scripts/ci/run-fast-checks.sh
make stage1-smoke
