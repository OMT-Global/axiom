#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
script="$repo_root/scripts/ci/run-direct-native-runtime-abi-evidence.sh"
makefile="$repo_root/Makefile"

[[ -x "$script" ]] || {
  echo "missing executable direct native runtime ABI evidence runner: $script" >&2
  exit 1
}

grep -Fq 'check-direct-native-runtime-abi.py --json' "$script" || {
  echo "evidence runner must validate the direct native runtime ABI manifest" >&2
  exit 1
}

grep -Fq -- '--test cranelift_backend' "$script" || {
  echo "evidence runner must execute the Cranelift backend evidence suite" >&2
  exit 1
}

grep -Fq 'AXIOM_DIRECT_NATIVE_RUNTIME_ABI_TEST_FILTER' "$script" || {
  echo "evidence runner must expose a focused test filter for local repair loops" >&2
  exit 1
}

grep -Fq 'stage1-direct-native-runtime-abi-evidence:' "$makefile" || {
  echo "Makefile must expose stage1-direct-native-runtime-abi-evidence" >&2
  exit 1
}

echo "direct native runtime ABI evidence runner regression cases passed"
