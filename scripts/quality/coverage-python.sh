#!/usr/bin/env bash
set -euo pipefail

out_dir="${QUALITY_OUT_DIR:-.quality}"
python_bin="${PYTHON:-python3}"
coverage_dir="$out_dir/coverage"
mkdir -p "$coverage_dir"

"$python_bin" -m coverage erase
"$python_bin" -m coverage run -m unittest discover -v
"$python_bin" -m coverage json --pretty-print -o "$coverage_dir/python.json"
"$python_bin" -m coverage report
