#!/usr/bin/env bash
set -euo pipefail

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

all_closed="$tmpdir/issues-closed.txt"
one_open="$tmpdir/issues-open.txt"
missing_state="$tmpdir/issues-missing.txt"

cat > "$all_closed" <<'STATES'
266 CLOSED
267 CLOSED
268 CLOSED
269 CLOSED
270 CLOSED
271 CLOSED
STATES

cat > "$one_open" <<'STATES'
266 CLOSED
267 CLOSED
268 OPEN
269 CLOSED
270 CLOSED
271 CLOSED
STATES

cat > "$missing_state" <<'STATES'
266 CLOSED
267 CLOSED
268 CLOSED
269 CLOSED
270 CLOSED
STATES

assert_success() {
  local name="$1"
  shift
  local output="$tmpdir/$name.out"
  if ! "$@" >"$output" 2>&1; then
    echo "$name: expected success" >&2
    cat "$output" >&2
    exit 1
  fi
}

assert_failure_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local output="$tmpdir/$name.out"
  if "$@" >"$output" 2>&1; then
    echo "$name: expected failure" >&2
    exit 1
  fi
  if ! grep -Fq "$expected" "$output"; then
    echo "$name: missing expected output: $expected" >&2
    cat "$output" >&2
    exit 1
  fi
}

assert_success all_closed \
  bash scripts/ci/check-python-exit-readiness.sh --json \
    --issue-state-file "$all_closed" \
    --require-issue-states

assert_failure_contains one_open "issue #268 is OPEN" \
  bash scripts/ci/check-python-exit-readiness.sh --json \
    --issue-state-file "$one_open" \
    --require-issue-states

assert_failure_contains missing_state "issue #271 state is unavailable" \
  bash scripts/ci/check-python-exit-readiness.sh --json \
    --issue-state-file "$missing_state" \
    --require-issue-states

echo "check-python-exit-readiness regression cases passed"
