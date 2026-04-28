import os
import subprocess
import sys
import tempfile
import textwrap
from pathlib import Path
from unittest import TestCase


ROOT = Path(__file__).resolve().parents[1]
PREFLIGHT = ROOT / "scripts" / "ci" / "preflight-test-collection.sh"


class PreflightTestCollectionTests(TestCase):
    def run_preflight(self, test_source: str) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as tmp:
            tests_dir = Path(tmp) / "tests"
            tests_dir.mkdir()
            (tests_dir / "test_sample.py").write_text(
                textwrap.dedent(test_source),
                encoding="utf-8",
            )
            env = {
                **os.environ,
                "PYTHON": sys.executable,
                "TEST_DISCOVERY_START_DIR": str(tests_dir),
            }
            return subprocess.run(
                ["bash", str(PREFLIGHT)],
                cwd=ROOT,
                env=env,
                text=True,
                capture_output=True,
                check=False,
            )

    def test_import_time_collection_failure_exits_nonzero(self) -> None:
        result = self.run_preflight(
            """
            import missing_axiom_preflight_dependency
            """
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("import/collection failures", result.stderr)
        self.assertIn("test_sample", result.stderr)

    def test_valid_tests_collect_without_running_assertions(self) -> None:
        result = self.run_preflight(
            """
            import unittest


            class SampleTests(unittest.TestCase):
                def test_not_executed_during_collection(self):
                    self.fail("collection preflight should not run test bodies")
            """
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("imported 1 tests without collection failures", result.stdout)
