#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
workflow="$repo_root/.github/workflows/pr-fast-ci.yml"

if [[ ! -f "$workflow" ]]; then
  echo "missing workflow: $workflow" >&2
  exit 1
fi

section="$({
  awk '
    /^  validate-pr-description:$/ { in_job=1; print; next }
    in_job && /^  [A-Za-z0-9_-]+:$/ { exit }
    in_job { print }
  ' "$workflow"
})"

if [[ -z "$section" ]]; then
  echo "validate-pr-description job is missing from pr-fast-ci workflow" >&2
  exit 1
fi

checkout_line=$(printf '%s\n' "$section" | nl -ba | grep 'actions/checkout@' | head -n1 | awk '{print $1}')
run_line=$(printf '%s\n' "$section" | nl -ba | grep 'bash scripts/ci/validate-pr-description.sh' | head -n1 | awk '{print $1}')

if [[ -z "$checkout_line" ]]; then
  echo "validate-pr-description job must checkout the repo before running validation" >&2
  exit 1
fi

if [[ -z "$run_line" ]]; then
  echo "validate-pr-description job must run the PR description validation script" >&2
  exit 1
fi

if (( checkout_line >= run_line )); then
  echo "validate-pr-description job must checkout the repo before running validation" >&2
  exit 1
fi

echo "pr-fast-ci workflow validation passed"
