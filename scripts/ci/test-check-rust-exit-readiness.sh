#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
script="$repo_root/scripts/ci/check-rust-exit-readiness.sh"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

case_dir="$temp_dir/repo"
mkdir -p \
  "$case_dir/docs" \
  "$case_dir/scripts/ci" \
  "$case_dir/stage1/runtime-abi" \
  "$case_dir/stage1/compiler-contracts/snapshots" \
  "$case_dir/stage1/crates/axiomc/tests" \
  "$case_dir/stage1/crates/axiomc-backend-cranelift/src"
cp "$script" "$case_dir/scripts/ci/check-rust-exit-readiness.sh"
cp "$repo_root/scripts/ci/check-direct-native-runtime-abi.py" "$case_dir/scripts/ci/check-direct-native-runtime-abi.py"
cp "$repo_root/scripts/ci/run-direct-native-runtime-abi-evidence.sh" "$case_dir/scripts/ci/run-direct-native-runtime-abi-evidence.sh"
cp "$repo_root/docs/rust-exit-readiness.md" "$case_dir/docs/rust-exit-readiness.md"
cp "$repo_root/docs/rust-exit-readiness.json" "$case_dir/docs/rust-exit-readiness.json"
cp "$repo_root/stage1/runtime-abi/direct-native-v0.json" "$case_dir/stage1/runtime-abi/direct-native-v0.json"
cp "$repo_root/stage1/compiler-contracts/snapshots/command-lsp.json" "$case_dir/stage1/compiler-contracts/snapshots/command-lsp.json"
cp "$repo_root/stage1/compiler-contracts/snapshots/mir-backend.json" "$case_dir/stage1/compiler-contracts/snapshots/mir-backend.json"
cp "$repo_root/stage1/crates/axiomc/tests/cranelift_backend.rs" "$case_dir/stage1/crates/axiomc/tests/cranelift_backend.rs"
cp "$repo_root/stage1/crates/axiomc-backend-cranelift/src/lib.rs" "$case_dir/stage1/crates/axiomc-backend-cranelift/src/lib.rs"
cat >"$case_dir/Makefile" <<'MAKE'
rust-exit-readiness:
	bash scripts/ci/check-rust-exit-readiness.sh --json

rust-exit-readiness-github:
	bash scripts/ci/check-rust-exit-readiness.sh --json --require-issue-states

rust-exit-readiness-test:
	bash scripts/ci/test-check-rust-exit-readiness.sh
MAKE

make_abi_ready() {
  python3 - "$case_dir/stage1/runtime-abi/direct-native-v0.json" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, encoding="utf-8") as handle:
    contract = json.load(handle)

contract["status"] = "implemented"
for row in contract["value_features"] + contract["capability_shims"]:
    row["status"] = "implemented"
    row.pop("blockers", None)
    row["evidence"] = ["stage1/crates/axiomc/tests/cranelift_backend.rs"]
    row["runtime_evidence"] = ["stage1/crates/axiomc/tests/cranelift_backend.rs"]

with open(path, "w", encoding="utf-8") as handle:
    json.dump(contract, handle)
PY
}

set_blockers() {
  python3 - "$case_dir/docs/rust-exit-readiness.json" "$@" <<'PY'
import json
import sys

path = sys.argv[1]
issues = [int(issue) for issue in sys.argv[2:]]
lane_by_issue = {1124: "backend", 731: "tooling", 721: "bootstrap"}
check_by_issue = {
    1124: "Complete remaining direct-native value-feature ABI rows and make the runtime ABI report ready.",
    731: "axiomc lsp driver is AxiOM-owned and no longer depends on Rust for the LSP path.",
    721: "Rust bootstrap is no longer needed for the official compiler and release path.",
}
with open(path, encoding="utf-8") as handle:
    payload = json.load(handle)
payload["blockingIssues"] = [
    {
        "issue": issue,
        "lane": lane_by_issue[issue],
        "check": check_by_issue[issue],
    }
    for issue in issues
]
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle)
PY
}

make_abi_ready

cat >"$temp_dir/open-issues.txt" <<'ISSUES'
1124 OPEN
731 OPEN
721 OPEN
ISSUES

(
  cd "$case_dir"
  if bash scripts/ci/check-rust-exit-readiness.sh --json --issue-state-file "$temp_dir/open-issues.txt" >"$temp_dir/blocked.json" 2>"$temp_dir/blocked.err"; then
    echo "expected readiness check to fail while live blocking issues remain listed" >&2
    exit 1
  fi
  python3 - "$temp_dir/blocked.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)

assert payload["schema"] == "axiom.rust_exit.readiness.v1"
assert payload["ready"] is False
statuses = {check["name"]: check["status"] for check in payload["checks"]}
assert statuses["readiness_doc_present"] == "pass"
assert statuses["readiness_manifest_valid"] == "pass"
assert statuses["readiness_blockers_current"] == "pass"
assert statuses["readiness_blockers_clear"] == "fail"
assert statuses["rust_exit_issue_1124_open"] == "pass"
assert statuses["rust_exit_issue_731_open"] == "pass"
assert statuses["rust_exit_issue_721_open"] == "pass"
assert statuses["direct_native_runtime_abi_ready"] == "pass"
assert statuses["command_lsp_release_boundary"] == "pass"
assert statuses["mir_backend_direct_native_boundary"] == "pass"
PY
)

set_blockers

(
  cd "$case_dir"
  if ! bash scripts/ci/check-rust-exit-readiness.sh --json --issue-state-file "$temp_dir/open-issues.txt" >"$temp_dir/ready.json" 2>"$temp_dir/ready.err"; then
    echo "expected readiness check to pass once the blocker manifest is empty and the ABI report is ready" >&2
    exit 1
  fi
  python3 - "$temp_dir/ready.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)

assert payload["ready"] is True
statuses = {check["name"]: check["status"] for check in payload["checks"]}
assert statuses["readiness_blockers_current"] == "pass"
assert statuses["readiness_blockers_clear"] == "pass"
assert statuses["direct_native_runtime_abi_ready"] == "pass"
assert statuses["command_lsp_release_boundary"] == "pass"
assert statuses["mir_backend_direct_native_boundary"] == "pass"
PY
)

set_blockers 1124 731 721
cat >"$temp_dir/closed-issues.txt" <<'ISSUES'
1124 CLOSED
731 CLOSED
721 CLOSED
ISSUES

(
  cd "$case_dir"
  if bash scripts/ci/check-rust-exit-readiness.sh --json --issue-state-file "$temp_dir/closed-issues.txt" >"$temp_dir/stale.json" 2>"$temp_dir/stale.err"; then
    echo "expected readiness check to fail when listed blockers are already closed" >&2
    exit 1
  fi
  python3 - "$temp_dir/stale.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)

statuses = {check["name"]: check["status"] for check in payload["checks"]}
assert statuses["readiness_blockers_current"] == "fail"
assert statuses["readiness_blockers_clear"] == "fail"
assert statuses["rust_exit_issue_1124_open"] == "fail"
assert statuses["rust_exit_issue_731_open"] == "fail"
assert statuses["rust_exit_issue_721_open"] == "fail"
PY
)

set_blockers
python3 - "$case_dir/stage1/compiler-contracts/snapshots/command-lsp.json" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, encoding="utf-8") as handle:
    payload = json.load(handle)
payload["official_release"]["requires_cargo"] = True
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle)
PY

(
  cd "$case_dir"
  if bash scripts/ci/check-rust-exit-readiness.sh --json --issue-state-file "$temp_dir/open-issues.txt" >"$temp_dir/cargo-boundary.json" 2>"$temp_dir/cargo-boundary.err"; then
    echo "expected readiness check to fail when command/LSP release requires Cargo" >&2
    exit 1
  fi
  python3 - "$temp_dir/cargo-boundary.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)
statuses = {check["name"]: check["status"] for check in payload["checks"]}
assert statuses["command_lsp_release_boundary"] == "fail"
PY
)

cp "$repo_root/stage1/compiler-contracts/snapshots/command-lsp.json" "$case_dir/stage1/compiler-contracts/snapshots/command-lsp.json"

python3 - "$case_dir/stage1/compiler-contracts/snapshots/mir-backend.json" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, encoding="utf-8") as handle:
    payload = json.load(handle)
for target in payload["targets"]:
    if target["id"] == "axiom://target/stage1-direct-native":
        target["primary_artifacts"].append("rust_source")
        break
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle)
PY

(
  cd "$case_dir"
  if bash scripts/ci/check-rust-exit-readiness.sh --json --issue-state-file "$temp_dir/open-issues.txt" >"$temp_dir/mir-boundary.json" 2>"$temp_dir/mir-boundary.err"; then
    echo "expected readiness check to fail when direct-native target emits rust_source" >&2
    exit 1
  fi
  python3 - "$temp_dir/mir-boundary.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    payload = json.load(handle)
statuses = {check["name"]: check["status"] for check in payload["checks"]}
assert statuses["mir_backend_direct_native_boundary"] == "fail"
PY
)

echo "check-rust-exit-readiness regression cases passed"
