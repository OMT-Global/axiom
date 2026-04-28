#!/usr/bin/env bash
set -euo pipefail

python_bin="${PYTHON:-python3}"
start_dir="${TEST_DISCOVERY_START_DIR:-tests}"

"$python_bin" - "$start_dir" <<'PY'
import sys
import unittest

start_dir = sys.argv[1]

def iter_tests(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from iter_tests(item)
        else:
            yield item


suite = unittest.defaultTestLoader.discover(start_dir)
tests = list(iter_tests(suite))
failed_imports = [
    str(test)
    for test in tests
    if test.__class__.__module__ == "unittest.loader"
    and test.__class__.__name__ == "_FailedTest"
]

if failed_imports:
    print("unittest discovery found import/collection failures:", file=sys.stderr)
    for test_name in failed_imports:
        print(f"  - {test_name}", file=sys.stderr)
    sys.exit(1)

if not tests:
    print(f"unittest discovery found no tests under {start_dir}/", file=sys.stderr)
    sys.exit(1)

print(f"unittest discovery imported {len(tests)} tests without collection failures.")
PY
