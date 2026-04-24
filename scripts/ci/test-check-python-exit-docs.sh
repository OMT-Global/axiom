#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_script="$repo_root/scripts/ci/check-python-exit-docs.sh"

if [[ ! -f "$source_script" ]]; then
  echo "missing source script: $source_script" >&2
  exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
legacy_invocation="python -m axi""om"
python_unittest="python -m unit""test"

assert_success() {
  local case_name="$1"
  local status="$2"
  local output_path="$3"

  if [[ "$status" -ne 0 ]]; then
    echo "$case_name: expected success" >&2
    cat "$output_path" >&2
    exit 1
  fi
}

assert_failure_contains() {
  local case_name="$1"
  local status="$2"
  local output_path="$3"
  local expected="$4"

  if [[ "$status" -eq 0 ]]; then
    echo "$case_name: expected failure" >&2
    exit 1
  fi

  if ! grep -Fq "$expected" "$output_path"; then
    echo "$case_name: missing expected output: $expected" >&2
    cat "$output_path" >&2
    exit 1
  fi
}

setup_case_repo() {
  local case_dir="$1"

  mkdir -p "$case_dir/docs" "$case_dir/scripts/ci"
  cp "$source_script" "$case_dir/scripts/ci/check-python-exit-docs.sh"
  cp "$repo_root/docs/python-exit-vm-disposition.md" "$case_dir/docs/python-exit-vm-disposition.md"
  cp "$repo_root/docs/python-exit-parity-gate.md" "$case_dir/docs/python-exit-parity-gate.md"
  : > "$case_dir/Makefile"
  : > "$case_dir/project.bootstrap.yaml"

  (
    cd "$case_dir"
    git init -q
    git config user.name "Ares"
    git config user.email "ares@example.com"
    git add docs scripts
    git commit -q -m "fixture"
  )
}

run_case() {
  local case_name="$1"
  local expected_status="$2"
  local expected_text="${3:-}"
  local case_dir="$tmpdir/$case_name"
  local output_path="$tmpdir/$case_name.out"
  local status=0

  setup_case_repo "$case_dir"

  case "$case_name" in
    excluded_docs_allow_legacy_strings)
      printf '%s\n' "$legacy_invocation" >> "$case_dir/docs/python-exit-vm-disposition.md"
      printf '%s\n' "$legacy_invocation" >> "$case_dir/docs/python-exit-parity-gate.md"
      ;;
    rejects_legacy_invocation_in_user_docs)
      printf '%s\n' "$legacy_invocation" > "$case_dir/README.md"
      ;;
    rejects_blocked_parity_rows)
      awk '
        {
          if (!inserted && $0 == "There are no `blocked` rows in the current matrix.") {
            print "| synthetic blocked case | `blocked` | Linked child issue is still open. |"
            print ""
            inserted = 1
          }
          print
        }
      ' "$case_dir/docs/python-exit-parity-gate.md" > "$case_dir/docs/python-exit-parity-gate.md.tmp"
      mv "$case_dir/docs/python-exit-parity-gate.md.tmp" "$case_dir/docs/python-exit-parity-gate.md"
      ;;
    rejects_python_unittest_gate)
      mkdir -p "$case_dir/.github/workflows"
      printf '%s\n' "$python_unittest" > "$case_dir/.github/workflows/pr-fast-ci.yml"
      ;;
    rejects_tracked_stage0_files)
      mkdir -p "$case_dir/axiom"
      printf '%s\n' 'print("legacy")' > "$case_dir/axiom/legacy.py"
      ;;
    *)
      echo "unknown case: $case_name" >&2
      exit 1
      ;;
  esac

  (
    cd "$case_dir"
    git add .
    set +e
    bash scripts/ci/check-python-exit-docs.sh >"$output_path" 2>&1
    status=$?
    set -e
    echo "$status" > "$tmpdir/$case_name.status"
  )

  status="$(cat "$tmpdir/$case_name.status")"

  if [[ "$expected_status" == "success" ]]; then
    assert_success "$case_name" "$status" "$output_path"
  else
    assert_failure_contains "$case_name" "$status" "$output_path" "$expected_text"
  fi
}

run_case excluded_docs_allow_legacy_strings success
run_case rejects_legacy_invocation_in_user_docs failure "user-facing docs still instruct users to run $legacy_invocation"
run_case rejects_blocked_parity_rows failure "Python exit parity matrix has blocked rows"
run_case rejects_python_unittest_gate failure "CI still uses Python unittest as a language/runtime correctness gate"
run_case rejects_tracked_stage0_files failure "Python stage0 source, tests, or packaging files are still tracked"

echo "check-python-exit-docs regression cases passed"
