#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
script="$repo_root/scripts/ci/check-package-graph-boundary.py"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

python3 "$script" --json >"$temp_dir/result.json"

python3 - "$temp_dir/result.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)

assert payload["schema"] == "axiom.compiler.package_graph.v1"
assert payload["ok"] is True
assert payload["packages"] == 3
PY

python3 - "$repo_root/stage1/compiler-contracts/snapshots/package-graph.json" "$temp_dir/unexpected-field.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)

payload["outputs"]["packages"][0]["unexpected"] = "drift"

with open(sys.argv[2], "w", encoding="utf-8") as handle:
    json.dump(payload, handle)
PY

if python3 "$script" --snapshot "$temp_dir/unexpected-field.json" >"$temp_dir/unexpected.out" 2>"$temp_dir/unexpected.err"; then
  echo "expected schema-invalid package graph output to fail" >&2
  exit 1
fi

grep -q "unexpected fields" "$temp_dir/unexpected.err"

python3 - "$repo_root/stage1/compiler-contracts/snapshots/package-graph.json" "$temp_dir/cargo-derived.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)

payload["outputs"]["packages"][0]["source"] = "Cargo.toml"

with open(sys.argv[2], "w", encoding="utf-8") as handle:
    json.dump(payload, handle)
PY

if python3 "$script" --snapshot "$temp_dir/cargo-derived.json" >"$temp_dir/cargo.out" 2>"$temp_dir/cargo.err"; then
  echo "expected Cargo-derived package graph output to fail" >&2
  exit 1
fi

grep -q "Cargo-derived" "$temp_dir/cargo.err"

python3 - "$repo_root/stage1/compiler-contracts/snapshots/package-graph.json" "$temp_dir/stale-lockfile.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)

payload["outputs"]["lockfile_integrity"]["packages"][1]["version"] = "9.9.9"

with open(sys.argv[2], "w", encoding="utf-8") as handle:
    json.dump(payload, handle)
PY

if python3 "$script" --snapshot "$temp_dir/stale-lockfile.json" >"$temp_dir/stale.out" 2>"$temp_dir/stale.err"; then
  echo "expected stale lockfile integrity fixture to fail" >&2
  exit 1
fi

grep -q "lockfile_integrity packages" "$temp_dir/stale.err"

echo "check-package-graph-boundary regression cases passed"
