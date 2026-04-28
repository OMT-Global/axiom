#!/usr/bin/env bash
set -euo pipefail

if [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

if ! cargo mutants --version >/dev/null 2>&1; then
  echo "cargo-mutants is required. Install it with: cargo install cargo-mutants --locked" >&2
  exit 127
fi

cargo mutants \
  --manifest-path stage1/Cargo.toml \
  "$@"
