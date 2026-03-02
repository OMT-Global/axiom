from __future__ import annotations

import subprocess
import sys
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
PROGRAMS_DIR = ROOT / "tests" / "programs"


class CliParityTests(unittest.TestCase):
    def _run_cli(self, args: list[str], *, cwd: Path) -> subprocess.CompletedProcess[str]:
        proc = subprocess.run(
            [sys.executable, "-m", "axiom", *args],
            capture_output=True,
            text=True,
            cwd=str(cwd),
        )
        self.assertEqual(
            proc.returncode,
            0,
            msg=f"{' '.join(args)} failed: {proc.stdout}\n{proc.stderr}",
        )
        return proc

    def test_programs_execute_interpreter_vm_and_run_in_parity(self) -> None:
        for path in sorted(PROGRAMS_DIR.glob("*.ax")):
            expected = (path.with_suffix(".out")).read_text(encoding="utf-8")
            interp_out = self._run_cli(["interp", str(path)], cwd=ROOT).stdout
            self.assertEqual(interp_out, expected, f"interp mismatch: {path.name}")

            with tempfile.TemporaryDirectory() as td:
                bc = Path(td) / f"{path.stem}.axb"
                self._run_cli(["compile", str(path), "-o", str(bc)], cwd=ROOT)
                vm_out = self._run_cli(["vm", str(bc)], cwd=ROOT).stdout
                self.assertEqual(vm_out, expected, f"vm mismatch: {path.name}")

            run_out = self._run_cli(["run", str(path)], cwd=ROOT).stdout
            self.assertEqual(run_out, expected, f"run mismatch: {path.name}")

    def test_check_reports_ok(self) -> None:
        path = PROGRAMS_DIR / "arith.ax"
        proc = self._run_cli(["check", str(path)], cwd=ROOT)
        self.assertIn("OK", proc.stderr)


if __name__ == "__main__":
    unittest.main()
