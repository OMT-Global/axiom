#!/usr/bin/env bash
set -euo pipefail

decision_doc="docs/python-exit-vm-disposition.md"
parity_doc="docs/python-exit-parity.md"

if [[ ! -f "$decision_doc" ]]; then
  echo "missing $decision_doc" >&2
  exit 1
fi

if [[ ! -f "$parity_doc" ]]; then
  echo "missing $parity_doc" >&2
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

parity_patterns=(
  "Parent issue: [#265]"
  "Governing issue: [#266]"
  "| Python-facing surface | Status | Rust-only path or disposition | Verification |"
  '| `python -m axi''om check` | replaced |'
  '| `python -m axi''om compile` | replaced |'
  '| `python -m axi''om interp` | retired |'
  '| `python -m axi''om vm` | retired |'
  '| `python -m axi''om repl` | retired |'
  '| `python -m axi''om pkg init` | replaced |'
  '| `python -m axi''om pkg build` | replaced |'
  '| `python -m axi''om pkg check` | replaced |'
  '| `python -m axi''om pkg run` | replaced |'
  '| `python -m axi''om pkg clean` | retired |'
  '| `python -m axi''om pkg manifest` | retired |'
  '| `python -m axi''om host list` | retired |'
  '| `python -m axi''om host describe` | retired |'
  "| Python language conformance tests | ported |"
  "| Python package/runtime examples | ported |"
  "| Python bytecode format and disassembler | retired |"
  "There are no blocked rows in the current matrix."
  "make stage1-test"
  "make stage1-conformance"
  "make stage1-smoke"
  "bash scripts/check-detect-secrets.sh --all-files"
)

for pattern in "${parity_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$parity_doc"; then
    echo "missing Python exit parity text: $pattern" >&2
    exit 1
  fi
done

legacy_invocation="python -m axi""om"

if rg -n "$legacy_invocation" README.md docs scripts \
  --glob '*.md' \
  --glob '*.sh' \
  --glob '!docs/python-exit-parity.md' \
  --glob '!docs/python-exit-vm-disposition.md'; then
  echo "user-facing docs still instruct users to run $legacy_invocation" >&2
  exit 1
fi

python_unittest="python -m unit""test"

if rg -n "$python_unittest" .github scripts Makefile project.bootstrap.yaml; then
  echo "CI still uses Python unittest as a language/runtime correctness gate" >&2
  exit 1
fi

stage0_pathspecs=(
  ':(glob)axiom/**'
  ':(glob)tests/**'
  ':(glob)requirements*.txt'
  '.python-version'
  'Pipfile'
  'Pipfile.lock'
  'poetry.lock'
  'pyproject.toml'
  'setup.cfg'
  'setup.py'
  'tox.ini'
)

tracked_stage0_files="$(git ls-files -- "${stage0_pathspecs[@]}")"

if [[ -n "$tracked_stage0_files" ]]; then
  echo "Python stage0 source, tests, or packaging files are still tracked" >&2
  printf '%s\n' "$tracked_stage0_files" >&2
  exit 1
fi
