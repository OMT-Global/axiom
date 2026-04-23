#!/usr/bin/env bash
set -euo pipefail

decision_doc="docs/python-exit-vm-disposition.md"

if [[ ! -f "$decision_doc" ]]; then
  echo "missing $decision_doc" >&2
  exit 1
fi

required_patterns=(
  "Python interpreter | Retire"
  "Python bytecode compiler | Retire"
  "Python bytecode format | Preserve only as historical material"
  "Python bytecode VM | Retire"
  "Python disassembler | Retire"
  "There will be no Rust port of the Python bytecode interpreter or VM"
  "Legacy module command | Disposition"
  '`check` | Use `axiomc check <package>`'
  '`compile` | Use `axiomc build <package>`'
  '`interp` | Retire'
  '`vm` | Retire with the bytecode VM'
  '`repl` | Retire'
  '`pkg init` | Use `axiomc new <path>`'
  '`pkg build` | Use `axiomc build <package>`'
  '`pkg check` | Use `axiomc check <package>`'
  '`pkg run` | Use `axiomc run <package>`'
  'package tests | Use `axiomc test <package>`'
  '`pkg clean` | Retire'
  '`pkg manifest` | Retire as a separate command'
  '`host list` | Retire'
  '`host describe` | Retire'
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$decision_doc"; then
    echo "missing Python exit decision text: $pattern" >&2
    exit 1
  fi
done

legacy_invocation="python -m axi""om"

if rg -n "$legacy_invocation" README.md docs scripts --glob '*.md' --glob '*.sh'; then
  echo "user-facing docs still instruct users to run $legacy_invocation" >&2
  exit 1
fi
