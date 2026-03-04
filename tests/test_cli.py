from __future__ import annotations

import subprocess
import sys
from pathlib import Path
import hashlib
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
        self.assertEqual([entry["name"] for entry in payload], sorted(names))
        version_entry = next(e for e in payload if e["name"] == "version")
        self.assertEqual(version_entry["arity"], 0)
        self.assertFalse(version_entry["side_effecting"])

        safe_proc = self._run_cli(["host", "list", "--safe-only"], cwd=ROOT)
        safe_payload = json.loads(safe_proc.stdout)
        self.assertTrue(isinstance(safe_payload, list))
        self.assertTrue(all(not entry["side_effecting"] for entry in safe_payload))
        safe_names = {entry["name"] for entry in safe_payload}
        self.assertIn("version", safe_names)
        self.assertNotIn("print", safe_names)

    def test_host_describe_command(self) -> None:
        proc = self._run_cli(["host", "describe"], cwd=ROOT)
        payload = json.loads(proc.stdout)
        self.assertEqual(payload["schema_version"], 1)
        self.assertIn("runtime_version_minor", payload)
        self.assertIn("capabilities", payload)
        self.assertIn("capabilities_signature", payload)
        caps = payload["capabilities"]
        self.assertTrue(isinstance(caps, list))
        names = {entry["name"] for entry in caps}
        self.assertIn("version", names)
        self.assertIn("print", names)
        self.assertIsInstance(payload["capabilities_signature"], str)
        self.assertEqual(
            payload["capabilities_signature"],
            hashlib.sha256(json.dumps(caps, sort_keys=True).encode("utf-8")).hexdigest(),
        )

        safe_proc = self._run_cli(["host", "describe", "--safe-only"], cwd=ROOT)
        safe_payload = json.loads(safe_proc.stdout)
        self.assertEqual(safe_payload["schema_version"], 1)
        safe_caps = safe_payload["capabilities"]
        self.assertTrue(isinstance(safe_caps, list))
        self.assertTrue(all(not entry["side_effecting"] for entry in safe_caps))
        self.assertNotEqual(payload["capabilities_signature"], safe_payload["capabilities_signature"])
        self.assertEqual(
            safe_payload["capabilities_signature"],
            hashlib.sha256(json.dumps(safe_caps, sort_keys=True).encode("utf-8")).hexdigest(),
        )
        # signature must be stable across repeated invocations with same registry state
        second_proc = self._run_cli(["host", "describe"], cwd=ROOT)
        second_payload = json.loads(second_proc.stdout)
        self.assertEqual(
            second_payload["capabilities_signature"],
            payload["capabilities_signature"],
        )

    def test_imported_modules_execute(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            mod = Path(td) / "math_module.ax"
            mod.write_text("fn add(a, b) {\n  return a + b\n}\n", encoding="utf-8")

            main = Path(td) / "main.ax"
            main.write_text('import "math_module"\nprint math_module.add(6, 7)\n', encoding="utf-8")

            proc = self._run_cli(["interp", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "13\n")
            proc = self._run_cli(["run", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "13\n")

    def test_imported_modules_execute_with_alias(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            mod = Path(td) / "math_module.ax"
            mod.write_text("fn add(a, b) {\n  return a + b\n}\n", encoding="utf-8")

            main = Path(td) / "main.ax"
            main.write_text(
                'import "math_module" as MATH\nprint MATH.add(9, 8)\n',
                encoding="utf-8",
            )

            proc = self._run_cli(["interp", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "17\n")
            proc = self._run_cli(["run", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "17\n")

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

    def test_package_init_with_allowed_host_calls(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(
                [
                    "pkg",
                    "init",
                    str(project),
                    "--name",
                    "demo",
                    "--allowed-host-call",
                    "print",
                    "--allowed-host-call",
                    "math.abs",
                ],
                cwd=ROOT,
            )
            manifest = json.loads((project / "axiom.pkg").read_text(encoding="utf-8"))
            self.assertEqual(manifest["allowed_host_calls"], ["print", "math.abs"])

    def test_package_check_command(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT)
            self.assertIn("OK", proc.stderr)

    def test_package_check_command_respects_host_effect_gating(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            (project / "src" / "main.ax").write_text("host.print(9)\n", encoding="utf-8")

            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = self._run_cli(
                ["pkg", "check", str(project), "--allow-host-side-effects"],
                cwd=ROOT,
            )
            self.assertIn("OK", proc.stderr)

    def test_package_check_command_fails_without_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            proc = self._run_cli(
                ["pkg", "check", str(project)], cwd=ROOT, expect_code=1
            )
            self.assertIn("missing package manifest", proc.stderr)

    def test_package_check_command_fails_with_invalid_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)

            manifest = project / "axiom.pkg"
            manifest.write_text("{invalid json}", encoding="utf-8")
            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid package manifest", proc.stderr)

    def test_package_check_rejects_unsafe_import_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            (project / "src" / "main.ax").write_text('import "../shared"\n', encoding="utf-8")
            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("parent traversal in import path", proc.stderr.lower())

    def test_package_check_rejects_absolute_import_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            (project / "src" / "main.ax").write_text('import "/etc/hosts"\n', encoding="utf-8")
            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("absolute import path", proc.stderr.lower())

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

    def test_package_build_with_nested_output_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)

            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["output"] = "nested/artifact.axb"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            main = project / manifest["main"]
            main.write_text("print 88\n", encoding="utf-8")

            self._run_cli(["pkg", "build", str(project)], cwd=ROOT)
            out = project / manifest["out_dir"] / "nested" / "artifact.axb"
            self.assertTrue(out.exists())

            vm_out = self._run_cli(["vm", str(out)], cwd=ROOT).stdout
            self.assertEqual(vm_out, "88\n")

    def test_package_check_rejects_disallowed_host_call(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["allowed_host_calls"] = ["abs"]
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            (project / manifest["main"]).write_text("host.print(9)\n", encoding="utf-8")

            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("not permitted by package policy", proc.stderr)

    def test_package_check_allows_host_call_when_manifest_allows(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["allowed_host_calls"] = ["print"]
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            (project / manifest["main"]).write_text("host.print(9)\n", encoding="utf-8")

            proc = self._run_cli(
                ["pkg", "check", str(project), "--allow-host-side-effects"], cwd=ROOT
            )
            self.assertIn("OK", proc.stderr)

            self._run_cli(["pkg", "build", str(project), "--allow-host-side-effects"], cwd=ROOT)
            out = project / manifest["out_dir"] / f"{manifest['name']}.axb"
            vm_out = self._run_cli(
                ["vm", str(out), "--allow-host-side-effects"], cwd=ROOT
            ).stdout
            self.assertEqual(vm_out, "9\n")

    def test_package_check_rejects_empty_host_allowlist(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["allowed_host_calls"] = []
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            (project / manifest["main"]).write_text("host.abs(-1)\n", encoding="utf-8")

            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("not permitted by package policy", proc.stderr)

    def test_package_check_allows_host_call_with_host_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["allowed_host_calls"] = ["host.print"]
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            (project / manifest["main"]).write_text("host.print(9)\n", encoding="utf-8")

            proc = self._run_cli(
                ["pkg", "check", str(project), "--allow-host-side-effects"], cwd=ROOT
            )
            self.assertIn("OK", proc.stderr)

            self._run_cli(["pkg", "run", str(project), "--allow-host-side-effects"], cwd=ROOT)

    def test_package_init_rejects_invalid_allowed_host_entry(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["allowed_host_calls"] = ["host."]
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            proc = self._run_cli(["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid allowed_host_calls entry", proc.stderr)

    def test_package_build_output_override_flag(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            main = project / "src" / "main.ax"
            main.write_text("print 9\n", encoding="utf-8")

            self._run_cli(
                [
                    "pkg",
                    "build",
                    str(project),
                    "--output",
                    "cli/nested.axb",
                ],
                cwd=ROOT,
            )
            out = project / "dist" / "cli" / "nested.axb"
            self.assertTrue(out.exists())

            vm_out = self._run_cli(["vm", str(out)], cwd=ROOT).stdout
            self.assertEqual(vm_out, "9\n")

    def test_package_build_rejects_absolute_output_override(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            (project / "src" / "main.ax").write_text("print 1\n", encoding="utf-8")

            proc = self._run_cli(
                [
                    "pkg",
                    "build",
                    str(project),
                    "--output",
                    "/tmp/cli.axb",
                ],
                cwd=ROOT,
                expect_code=1,
            )
            self.assertIn("absolute", proc.stderr.lower())

    def test_package_build_rejects_absolute_output_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["output"] = "/tmp/artifact.axb"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            (project / manifest["main"]).write_text("print 1\n", encoding="utf-8")

            proc = self._run_cli(["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("absolute", proc.stderr.lower())

    def test_package_build_rejects_parent_traversal_output_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["output"] = "../outside.axb"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            (project / manifest["main"]).write_text("print 1\n", encoding="utf-8")

            proc = self._run_cli(["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("parent traversal", proc.stderr.lower())

    def test_package_build_rejects_absolute_main_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["main"] = "/tmp/main.ax"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            proc = self._run_cli(["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("absolute", proc.stderr.lower())

    def test_package_build_rejects_parent_traversal_out_dir(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            self._run_cli(["pkg", "init", str(project), "--name", "demo"], cwd=ROOT)
            manifest_path = project / "axiom.pkg"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["out_dir"] = "../outside"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            proc = self._run_cli(["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("parent traversal", proc.stderr.lower())

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
