from __future__ import annotations

import json
from pathlib import Path
import shutil
import tempfile
import unittest

from axiom.errors import AxiomCompileError
from axiom.host import host_contract_metadata
from axiom.packaging import MAX_MANIFEST_BYTES, load_manifest
from tests.helpers import ROOT, init_temp_package, read_json, run_cli, write_json


class CliPackageTests(unittest.TestCase):
    def test_package_init_and_build(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            self.assertEqual(manifest["name"], "demo")

            main = project / "src" / "main.ax"
            main.write_text("print 12\n", encoding="utf-8")

            run_cli(self, ["pkg", "build", str(project)], cwd=ROOT)
            out = project / manifest["out_dir"] / "demo.axb"
            self.assertTrue(out.exists())

            vm_out = run_cli(self, ["vm", str(out)], cwd=ROOT).stdout
            self.assertEqual(vm_out, "12\n")

    def test_package_init_includes_host_contract_signature(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            expected_signature = host_contract_metadata()["capabilities_signature"]
            self.assertEqual(manifest["host_contract_signature"], expected_signature)

    def test_package_manifest_command(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            proc = run_cli(self, ["pkg", "manifest", str(project)], cwd=ROOT)
            payload = json.loads(proc.stdout)
            self.assertEqual(payload["name"], "demo")

    def test_load_manifest_allows_small_valid_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)

            manifest = load_manifest(project)

            self.assertEqual(manifest.name, "demo")

    def test_load_manifest_rejects_manifest_above_size_limit(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            project.mkdir(parents=True, exist_ok=True)
            manifest = project / "axiom.pkg"
            manifest.write_text("x" * (MAX_MANIFEST_BYTES + 1), encoding="utf-8")

            with self.assertRaises(AxiomCompileError) as cm:
                load_manifest(project)

            message = str(cm.exception)
            self.assertIn("too large", message)
            self.assertIn(str(MAX_MANIFEST_BYTES + 1), message)
            self.assertIn(str(MAX_MANIFEST_BYTES), message)

    def test_package_run_executes_main(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            (project / "src" / "main.ax").write_text("print 7\n", encoding="utf-8")
            proc = run_cli(self, ["pkg", "run", str(project)], cwd=ROOT)
            self.assertEqual(proc.stdout, "7\n")

    def test_package_build_and_run_string_program(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            (project / "src" / "main.ax").write_text('print "ship it"\n', encoding="utf-8")

            check_proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT)
            self.assertIn("OK", check_proc.stderr)

            build_proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT)
            self.assertIn("wrote", build_proc.stderr)

            run_proc = run_cli(self, ["pkg", "run", str(project)], cwd=ROOT)
            self.assertEqual(run_proc.stdout, "ship it\n")

    def test_codex_duo_system_demo_runs(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td) / "codex_duo_system"
            shutil.copytree(ROOT / "examples" / "codex_duo_system", project)

            check_proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT)
            self.assertIn("OK", check_proc.stderr)

            run_proc = run_cli(self, ["pkg", "run", str(project)], cwd=ROOT)
            self.assertEqual(
                run_proc.stdout,
                "+--------------------------------+\n"
                "| codex session a -> architect   |\n"
                "| codex session b -> builder     |\n"
                "+--------------------------------+\n"
                "session-a: defines the api contract and routing edge\n"
                "session-b: connects api-gateway to worker and audit\n"
                "system: relay-grid\n"
                "api-gateway -> job-worker -> audit-log\n"
                "shared result: relay-grid is ready\n",
            )

    def test_package_init_with_manifest_overrides(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            run_cli(
                self,
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
            manifest = read_json(project / "axiom.pkg")
            self.assertEqual(manifest["name"], "demo")
            self.assertEqual(manifest["version"], "2.0.0")
            self.assertEqual(manifest["main"], "src/app/main.ax")
            self.assertEqual(manifest["out_dir"], "build")
            self.assertEqual(manifest["output"], "bundle.axb")
            self.assertTrue((project / "src" / "app" / "main.ax").exists())

    def test_package_init_with_allowed_host_calls(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            run_cli(
                self,
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
            manifest = read_json(project / "axiom.pkg")
            self.assertEqual(manifest["allowed_host_calls"], ["print", "math.abs"])

    def test_package_check_rejects_host_contract_signature_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["host_contract_signature"] = "0" * 64
            write_json(project / "axiom.pkg", manifest)
            (project / manifest["main"]).write_text("print 9\n", encoding="utf-8")
            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("host_contract_signature mismatch", proc.stderr)

    def test_package_build_rejects_host_contract_signature_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["host_contract_signature"] = "0" * 64
            write_json(project / "axiom.pkg", manifest)
            proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("host_contract_signature mismatch", proc.stderr)

    def test_package_build_rejects_invalid_host_contract_signature(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["host_contract_signature"] = "bad"
            write_json(project / "axiom.pkg", manifest)
            proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid host_contract_signature", proc.stderr)

    def test_package_run_rejects_host_contract_signature_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["host_contract_signature"] = "0" * 64
            write_json(project / "axiom.pkg", manifest)
            proc = run_cli(self, ["pkg", "run", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("host_contract_signature mismatch", proc.stderr)

    def test_package_check_rejects_invalid_host_contract_signature(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["host_contract_signature"] = "not-a-valid-signature"
            write_json(project / "axiom.pkg", manifest)
            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid host_contract_signature", proc.stderr)

    def test_package_run_rejects_invalid_host_contract_signature(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["host_contract_signature"] = "bad"
            write_json(project / "axiom.pkg", manifest)
            proc = run_cli(self, ["pkg", "run", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid host_contract_signature", proc.stderr)

    def test_package_check_command(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT)
            self.assertIn("OK", proc.stderr)

    def test_package_check_command_respects_host_effect_gating(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            (project / "src" / "main.ax").write_text("host.print(9)\n", encoding="utf-8")

            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = run_cli(
                self,
                ["pkg", "check", str(project), "--allow-host-side-effects"],
                cwd=ROOT,
            )
            self.assertIn("OK", proc.stderr)

    def test_package_check_command_fails_without_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("missing package manifest", proc.stderr)

    def test_package_check_command_fails_with_invalid_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            manifest = project / "axiom.pkg"
            manifest.write_text("{invalid json}", encoding="utf-8")
            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid package manifest", proc.stderr)

    def test_package_check_rejects_unsafe_import_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            (project / "src" / "main.ax").write_text('import "../shared"\n', encoding="utf-8")
            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("parent traversal in import path", proc.stderr.lower())

    def test_package_check_rejects_absolute_import_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            (project / "src" / "main.ax").write_text('import "/etc/hosts"\n', encoding="utf-8")
            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("absolute import path", proc.stderr.lower())

    def test_package_init_force_rewrites_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project, name="first")
            run_cli(self, ["pkg", "init", str(project), "--name", "second", "--force"], cwd=ROOT)
            manifest = read_json(project / "axiom.pkg")
            self.assertEqual(manifest["name"], "second")

    def test_package_clean_removes_out_dir(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            (project / "src" / "main.ax").write_text("print 1\n", encoding="utf-8")

            run_cli(self, ["pkg", "build", str(project)], cwd=ROOT)
            self.assertTrue((project / "dist").exists())
            run_cli(self, ["pkg", "clean", str(project)], cwd=ROOT)
            self.assertFalse((project / "dist").exists())

    def test_package_init_requires_clean_directory(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            proc = run_cli(self, ["pkg", "init", str(project), "--name", "demo"], cwd=ROOT, expect_code=1)
            self.assertIn("package manifest already exists", proc.stderr)

    def test_package_build_with_custom_manifest_fields(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)

            manifest = {
                "name": "demo",
                "version": "9.9.9",
                "main": "src/app.ax",
                "out_dir": "build",
                "output": "artifact.axb",
            }
            write_json(project / "axiom.pkg", manifest)

            main = project / "src" / "app.ax"
            main.parent.mkdir(parents=True, exist_ok=True)
            main.write_text("print 42\n", encoding="utf-8")

            run_cli(self, ["pkg", "build", str(project)], cwd=ROOT)
            out = project / "build" / "artifact.axb"
            self.assertTrue(out.exists())
            self.assertEqual(run_cli(self, ["vm", str(out)], cwd=ROOT).stdout, "42\n")

    def test_package_build_with_nested_output_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["output"] = "nested/artifact.axb"
            write_json(project / "axiom.pkg", manifest)
            (project / manifest["main"]).write_text("print 88\n", encoding="utf-8")

            run_cli(self, ["pkg", "build", str(project)], cwd=ROOT)
            out = project / manifest["out_dir"] / "nested" / "artifact.axb"
            self.assertTrue(out.exists())
            self.assertEqual(run_cli(self, ["vm", str(out)], cwd=ROOT).stdout, "88\n")

    def test_package_check_rejects_disallowed_host_call(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["allowed_host_calls"] = ["abs"]
            write_json(project / "axiom.pkg", manifest)
            (project / manifest["main"]).write_text("host.print(9)\n", encoding="utf-8")

            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("not permitted by package policy", proc.stderr)

    def test_package_check_allows_host_call_when_manifest_allows(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["allowed_host_calls"] = ["print"]
            write_json(project / "axiom.pkg", manifest)
            (project / manifest["main"]).write_text("host.print(9)\n", encoding="utf-8")

            proc = run_cli(
                self,
                ["pkg", "check", str(project), "--allow-host-side-effects"],
                cwd=ROOT,
            )
            self.assertIn("OK", proc.stderr)

            run_cli(self, ["pkg", "build", str(project), "--allow-host-side-effects"], cwd=ROOT)
            out = project / manifest["out_dir"] / f"{manifest['name']}.axb"
            vm_out = run_cli(self, ["vm", str(out), "--allow-host-side-effects"], cwd=ROOT).stdout
            self.assertEqual(vm_out, "9\n")

    def test_package_check_rejects_empty_host_allowlist(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["allowed_host_calls"] = []
            write_json(project / "axiom.pkg", manifest)
            (project / manifest["main"]).write_text("host.abs(-1)\n", encoding="utf-8")

            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("not permitted by package policy", proc.stderr)

    def test_package_check_allows_host_call_with_host_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["allowed_host_calls"] = ["host.print"]
            write_json(project / "axiom.pkg", manifest)
            (project / manifest["main"]).write_text("host.print(9)\n", encoding="utf-8")

            proc = run_cli(
                self,
                ["pkg", "check", str(project), "--allow-host-side-effects"],
                cwd=ROOT,
            )
            self.assertIn("OK", proc.stderr)

            run_cli(self, ["pkg", "run", str(project), "--allow-host-side-effects"], cwd=ROOT)

    def test_package_init_rejects_invalid_allowed_host_entry(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["allowed_host_calls"] = ["host."]
            write_json(project / "axiom.pkg", manifest)

            proc = run_cli(self, ["pkg", "check", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid allowed_host_calls entry", proc.stderr)

    def test_package_build_output_override_flag(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            (project / "src" / "main.ax").write_text("print 9\n", encoding="utf-8")

            run_cli(self, ["pkg", "build", str(project), "--output", "cli/nested.axb"], cwd=ROOT)
            out = project / "dist" / "cli" / "nested.axb"
            self.assertTrue(out.exists())
            self.assertEqual(run_cli(self, ["vm", str(out)], cwd=ROOT).stdout, "9\n")

    def test_package_build_rejects_absolute_output_override(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            (project / "src" / "main.ax").write_text("print 1\n", encoding="utf-8")

            proc = run_cli(
                self,
                ["pkg", "build", str(project), "--output", "/tmp/cli.axb"],
                cwd=ROOT,
                expect_code=1,
            )
            self.assertIn("absolute", proc.stderr.lower())

    def test_package_build_rejects_absolute_output_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["output"] = "/tmp/artifact.axb"
            write_json(project / "axiom.pkg", manifest)
            (project / manifest["main"]).write_text("print 1\n", encoding="utf-8")

            proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("absolute", proc.stderr.lower())

    def test_package_build_rejects_parent_traversal_output_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["output"] = "../outside.axb"
            write_json(project / "axiom.pkg", manifest)
            (project / manifest["main"]).write_text("print 1\n", encoding="utf-8")

            proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("parent traversal", proc.stderr.lower())

    def test_package_build_rejects_absolute_main_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["main"] = "/tmp/main.ax"
            write_json(project / "axiom.pkg", manifest)

            proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("absolute", proc.stderr.lower())

    def test_package_build_rejects_parent_traversal_out_dir(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            manifest = init_temp_package(self, project)
            manifest["out_dir"] = "../outside"
            write_json(project / "axiom.pkg", manifest)

            proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("parent traversal", proc.stderr.lower())

    def test_package_build_fails_when_manifest_invalid_json(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            init_temp_package(self, project)
            manifest = project / "axiom.pkg"
            manifest.write_text("{", encoding="utf-8")
            proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("invalid package manifest", proc.stderr)

    def test_package_build_requires_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            project = Path(td)
            proc = run_cli(self, ["pkg", "build", str(project)], cwd=ROOT, expect_code=1)
            self.assertIn("missing package manifest", proc.stderr)


if __name__ == "__main__":
    unittest.main()
