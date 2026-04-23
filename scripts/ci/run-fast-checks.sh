#!/usr/bin/env bash
set -euo pipefail
make stage1-test
make stage1-conformance
make stage1-smoke
