from __future__ import annotations

import subprocess
import sys
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
PROGRAMS_DIR = ROOT / "tests" / "programs"


class CliParityTests(unittest.TestCase):
    def _run_cli(
        self, args: list[str], *, cwd: Path, expect_code: int = 0
    ) -> subprocess.CompletedProcess[str]:
        proc = subprocess.run(
            [sys.executable, "-m", "axiom", *args],
            capture_output=True,
            text=True,
            cwd=str(cwd),
        )
        self.assertEqual(
            proc.returncode,
            expect_code,
            msg=f"{' '.join(args)} failed (expected {expect_code}): {proc.stdout}\n{proc.stderr}",
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

    def test_check_can_allow_host_side_effects(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "host_print.ax"
            src.write_text("host.print(9)\n", encoding="utf-8")
            proc = self._run_cli(["check", str(src)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)
            proc = self._run_cli(
                ["check", str(src), "--allow-host-side-effects"], cwd=ROOT
            )
            self.assertIn("OK", proc.stderr)

    def test_host_bridge_side_effects_require_flag(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "host_print.ax"
            bc = Path(td) / "host_print.axb"
            src.write_text("host.print(9)\n", encoding="utf-8")

            proc = self._run_cli(["interp", str(src)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = self._run_cli(
                ["interp", str(src), "--allow-host-side-effects"], cwd=ROOT
            )
            self.assertEqual(proc.stdout, "9\n")

            proc = self._run_cli(
                ["compile", str(src), "-o", str(bc)], cwd=ROOT, expect_code=1
            )
            self.assertIn("side-effecting", proc.stderr)

            proc = self._run_cli(
                ["compile", str(src), "-o", str(bc), "--allow-host-side-effects"],
                cwd=ROOT,
            )
            self.assertIn("wrote", proc.stderr)

            proc = self._run_cli(["vm", str(bc)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = self._run_cli(["vm", str(bc), "--allow-host-side-effects"], cwd=ROOT)
            self.assertEqual(proc.stdout, "9\n")

            proc = self._run_cli(["run", str(src)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = self._run_cli(["run", str(src), "--allow-host-side-effects"], cwd=ROOT)
            self.assertEqual(proc.stdout, "9\n")

    def test_imported_modules_execute(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            mod = Path(td) / "math_module.ax"
            mod.write_text("fn add(a, b) {\n  return a + b\n}\n", encoding="utf-8")

            main = Path(td) / "main.ax"
            main.write_text('import "math_module"\nprint add(6, 7)\n', encoding="utf-8")

            proc = self._run_cli(["interp", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "13\n")
            proc = self._run_cli(["run", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "13\n")


if __name__ == "__main__":
    unittest.main()
