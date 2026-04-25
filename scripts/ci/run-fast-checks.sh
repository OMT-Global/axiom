#!/usr/bin/env bash
set -euo pipefail
bash scripts/ci/check-python-exit-docs.sh
make stage1-test
make stage1-conformance
make stage1-smoke
