#!/usr/bin/env bash
set -euo pipefail

out_dir="${QUALITY_OUT_DIR:-.quality}"
coverage_dir="$out_dir/coverage"
mkdir -p "$coverage_dir"

if [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "cargo-llvm-cov is required. Install it with: cargo install cargo-llvm-cov --locked" >&2
  exit 127
fi

cargo llvm-cov clean --manifest-path stage1/Cargo.toml --workspace
cargo llvm-cov \
  --manifest-path stage1/Cargo.toml \
  --workspace \
  --lcov \
  --output-path "$coverage_dir/rust.lcov"
