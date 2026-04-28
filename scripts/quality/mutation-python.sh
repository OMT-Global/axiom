#!/usr/bin/env bash
set -euo pipefail

python_bin="${PYTHON:-python3}"

if ! "$python_bin" -m mutmut --version >/dev/null 2>&1; then
  echo "mutmut is required. Install Python quality tooling with: $python_bin -m pip install -e '.[quality]'" >&2
  exit 127
fi

"$python_bin" -m mutmut run "$@"
