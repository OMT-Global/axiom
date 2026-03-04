from __future__ import annotations

import subprocess
import sys
from pathlib import Path
import tempfile
import json
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

    def test_host_list_command(self) -> None:
        proc = self._run_cli(["host", "list"], cwd=ROOT)
        payload = json.loads(proc.stdout)
        self.assertTrue(isinstance(payload, list))
        names = {entry["name"] for entry in payload}
        self.assertIn("version", names)
        self.assertIn("print", names)
        version_entry = next(e for e in payload if e["name"] == "version")
        self.assertEqual(version_entry["arity"], 0)
        self.assertFalse(version_entry["side_effecting"])

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

    def test_package_init_and_build(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)

            manifest_path = project / "axiom.pkg"
            self.assertTrue(manifest_path.exists())
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            self.assertEqual(manifest["name"], "demo")

            main = project / "src" / "main.ax"
            main.write_text("print 12\n", encoding="utf-8")

            self._run_cli(["pkg", "build", str(project)], cwd=ROOT)
            out = project / manifest["out_dir"] / "demo.axb"
            self.assertTrue(out.exists())

            vm_out = self._run_cli(["vm", str(out)], cwd=ROOT).stdout
            self.assertEqual(vm_out, "12\n")

    def test_package_manifest_command(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            proc = self._run_cli(["pkg", "manifest", str(project)], cwd=ROOT)
            payload = json.loads(proc.stdout)
            self.assertEqual(payload["name"], "demo")

    def test_package_run_executes_main(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            (project / "src" / "main.ax").write_text("print 7\n", encoding="utf-8")
            proc = self._run_cli(["pkg", "run", str(project)], cwd=ROOT)
            self.assertEqual(proc.stdout, "7\n")

    def test_package_init_with_manifest_overrides(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(
                [
                    "pkg",
                    "init",
                    str(project),
                    "--name",
                    "demo",
                    "--version",
                    "2.0.0",
                    "--main",
                    "src/app/main.ax",
                    "--out-dir",
                    "build",
                "--output",
                "bundle.axb",
            ],
            cwd=ROOT,
            )
            manifest = json.loads((project / "axiom.pkg").read_text(encoding="utf-8"))
            self.assertEqual(manifest["name"], "demo")
            self.assertEqual(manifest["version"], "2.0.0")
            self.assertEqual(manifest["main"], "src/app/main.ax")
            self.assertEqual(manifest["out_dir"], "build")
            self.assertEqual(manifest["output"], "bundle.axb")
            self.assertTrue((project / "src" / "app" / "main.ax").exists())

    def test_package_check_command(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT)
            self.assertIn("OK", proc.stderr)

    def test_package_init_force_rewrites_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "first"], cwd=ROOT)
            self._run_cli(["pkg", "init", str(project), "--name", "second", "--force"], cwd=ROOT)
            manifest = json.loads((project / "axiom.pkg").read_text(encoding="utf-8"))
            self.assertEqual(manifest["name"], "second")

    def test_package_clean_removes_out_dir(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            (project / "src" / "main.ax").write_text("print 1\n", encoding="utf-8")

            self._run_cli(["pkg", "build", str(project)], cwd=ROOT)
            self.assertTrue((project / "dist").exists())
            self._run_cli(["pkg", "clean", str(project)], cwd=ROOT)
            self.assertFalse((project / "dist").exists())

    def test_package_init_requires_clean_directory(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            proc = self._run_cli(
                ["pkg", "init", str(project), "--name", "demo"], cwd=ROOT, expect_code=1
            )
            self.assertIn("package manifest already exists", proc.stderr)

    def test_package_build_with_custom_manifest_fields(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)

            manifest_path = project / "axiom.pkg"
            manifest = {
                "name": "demo",
                "version": "9.9.9",
                "main": "src/app.ax",
                "out_dir": "build",
                "output": "artifact.axb",
            }
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            main = project / "src" / "app.ax"
            main.parent.mkdir(parents=True, exist_ok=True)
            main.write_text("print 42\n", encoding="utf-8")

            self._run_cli(["pkg", "build", str(project)], cwd=ROOT)
            out = project / "build" / "artifact.axb"
            self.assertTrue(out.exists())

            vm_out = self._run_cli(["vm", str(out)], cwd=ROOT).stdout
            self.assertEqual(vm_out, "42\n")

    def test_package_build_fails_when_manifest_invalid_json(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)

            manifest = project / "axiom.pkg"
            manifest.write_text("{", encoding="utf-8")
            proc = self._run_cli(["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid package manifest", proc.stderr)

    def test_package_build_requires_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            proc = self._run_cli(["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("missing package manifest", proc.stderr)


if __name__ == "__main__":
    unittest.main()
