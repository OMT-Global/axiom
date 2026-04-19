from __future__ import annotations

import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from tests.helpers import PROGRAMS_DIR, ROOT, run_cli, run_cli_json


class CliRuntimeTests(unittest.TestCase):
    def test_compile_demo_example_json_and_vm(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            bc = Path(td) / "compile_demo.axb"
            payload = run_cli_json(
                self,
                [
                    "compile",
                    str(ROOT / "examples" / "compile_demo.ax"),
                    "-o",
                    str(bc),
                    "--json",
                ],
                cwd=ROOT,
            )

            self.assertTrue(payload["ok"])
            self.assertEqual(payload["command"], "compile")
            self.assertEqual(
                payload["file"],
                str(ROOT / "examples" / "compile_demo.ax"),
            )
            self.assertEqual(payload["output"], str(bc))

            vm_proc = run_cli(self, ["vm", str(bc)], cwd=ROOT)
            self.assertEqual(
                vm_proc.stdout,
                "    /\\\n"
                "   /  \\\n"
                "  / /\\ \\\n"
                " / ____ \\\n"
                "/_/    \\_\\\n"
                "  AXIOM\n"
                "42\n",
            )

    def test_programs_execute_interpreter_vm_and_run_in_parity(self) -> None:
        for path in sorted(PROGRAMS_DIR.glob("*.ax")):
            expected = path.with_suffix(".out").read_text(encoding="utf-8")
            interp_out = run_cli(self, ["interp", str(path)], cwd=ROOT).stdout
            self.assertEqual(interp_out, expected, f"interp mismatch: {path.name}")

            with tempfile.TemporaryDirectory() as td:
                bc = Path(td) / f"{path.stem}.axb"
                run_cli(self, ["compile", str(path), "-o", str(bc)], cwd=ROOT)
                vm_out = run_cli(self, ["vm", str(bc)], cwd=ROOT).stdout
                self.assertEqual(vm_out, expected, f"vm mismatch: {path.name}")

            run_out = run_cli(self, ["run", str(path)], cwd=ROOT).stdout
            self.assertEqual(run_out, expected, f"run mismatch: {path.name}")

    def test_check_reports_ok(self) -> None:
        proc = run_cli(self, ["check", str(PROGRAMS_DIR / "arith.ax")], cwd=ROOT)
        self.assertIn("OK", proc.stderr)

    def test_check_reports_json_ok(self) -> None:
        payload = run_cli_json(
            self,
            ["check", str(PROGRAMS_DIR / "arith.ax"), "--json"],
            cwd=ROOT,
        )
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["command"], "check")
        self.assertEqual(payload["file"], str(PROGRAMS_DIR / "arith.ax"))
        self.assertTrue(payload["bytecode_ready"])
        self.assertEqual(payload["diagnostics"], [])

    def test_check_reports_json_error(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "host_print.ax"
            src.write_text("host.print(9)\n", encoding="utf-8")
            payload = run_cli_json(
                self,
                ["check", str(src), "--json"],
                cwd=ROOT,
                expect_code=1,
            )
        self.assertFalse(payload["ok"])
        self.assertEqual(payload["command"], "check")
        self.assertEqual(payload["file"], str(src))
        self.assertEqual(payload["error"]["kind"], "AxiomCompileError")
        self.assertIn("side-effecting", payload["error"]["message"])
        self.assertEqual(
            Path(payload["error"]["location"]["path"]).resolve(),
            src.resolve(),
        )

    def test_check_can_allow_host_side_effects(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "host_print.ax"
            src.write_text("host.print(9)\n", encoding="utf-8")
            proc = run_cli(self, ["check", str(src)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = run_cli(
                self,
                ["check", str(src), "--allow-host-side-effects"],
                cwd=ROOT,
            )
            self.assertIn("OK", proc.stderr)

    def test_repl_evaluates_expressions_with_type_information(self) -> None:
        proc = run_cli(
            self,
            ["repl"],
            cwd=ROOT,
            input_text="1 + 2\nlet name = \"axiom\"\nname + \"!\"\n:quit\n",
        )
        self.assertEqual(
            proc.stdout,
            "3 : int\n"
            "name : string = axiom\n"
            "axiom! : string\n",
        )
        self.assertEqual(proc.stderr, "")

    def test_repl_supports_multiline_functions_and_persistent_state(self) -> None:
        proc = run_cli(
            self,
            ["repl"],
            cwd=ROOT,
            input_text=(
                "fn add(a: int, b: int): int {\n"
                "return a + b\n"
                "}\n"
                "let total: int = add(20, 22)\n"
                "total\n"
                ":quit\n"
            ),
        )
        self.assertEqual(
            proc.stdout,
            "defined add : fn(int,int):int\n"
            "total : int = 42\n"
            "42 : int\n",
        )
        self.assertEqual(proc.stderr, "")

    def test_repl_supports_multiline_blocks(self) -> None:
        proc = run_cli(
            self,
            ["repl"],
            cwd=ROOT,
            input_text="if true {\nprint 5\n}\n:quit\n",
        )
        self.assertEqual(proc.stdout, "5\n")
        self.assertEqual(proc.stderr, "")

    def test_repl_help_lists_available_commands(self) -> None:
        proc = run_cli(
            self,
            ["repl"],
            cwd=ROOT,
            input_text=":help\n:quit\n",
        )
        self.assertEqual(proc.stdout, "Commands: :help, :quit, :exit\n")
        self.assertEqual(proc.stderr, "")

    def test_repl_recovers_after_errors_without_losing_prior_state(self) -> None:
        proc = run_cli(
            self,
            ["repl"],
            cwd=ROOT,
            input_text="let x: int = 7\nx + \"bad\"\nx + 1\n:quit\n",
        )
        self.assertEqual(proc.stdout, "x : int = 7\n8 : int\n")
        self.assertIn("operator '+' expects matching int or string operands", proc.stderr)

    def test_repl_host_side_effects_follow_flag(self) -> None:
        blocked = run_cli(
            self,
            ["repl"],
            cwd=ROOT,
            input_text="host.print(9)\n:quit\n",
        )
        self.assertEqual(blocked.stdout, "")
        self.assertIn("side-effecting", blocked.stderr)

        allowed = run_cli(
            self,
            ["repl", "--allow-host-side-effects"],
            cwd=ROOT,
            input_text="host.print(9)\n:quit\n",
        )
        self.assertEqual(allowed.stdout, "9\n0 : int\n")
        self.assertEqual(allowed.stderr, "")

    def test_host_bridge_side_effects_require_flag(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "host_print.ax"
            bc = Path(td) / "host_print.axb"
            src.write_text("host.print(9)\n", encoding="utf-8")

            proc = run_cli(self, ["interp", str(src)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = run_cli(
                self,
                ["interp", str(src), "--allow-host-side-effects"],
                cwd=ROOT,
            )
            self.assertEqual(proc.stdout, "9\n")

            proc = run_cli(
                self,
                ["compile", str(src), "-o", str(bc)],
                cwd=ROOT,
                expect_code=1,
            )
            self.assertIn("side-effecting", proc.stderr)

            proc = run_cli(
                self,
                ["compile", str(src), "-o", str(bc), "--allow-host-side-effects"],
                cwd=ROOT,
            )
            self.assertIn("wrote", proc.stderr)

            proc = run_cli(self, ["vm", str(bc)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = run_cli(self, ["vm", str(bc), "--allow-host-side-effects"], cwd=ROOT)
            self.assertEqual(proc.stdout, "9\n")

            proc = run_cli(self, ["run", str(src)], cwd=ROOT, expect_code=1)
            self.assertIn("side-effecting", proc.stderr)

            proc = run_cli(self, ["run", str(src), "--allow-host-side-effects"], cwd=ROOT)
            self.assertEqual(proc.stdout, "9\n")

    def test_host_list_command(self) -> None:
        proc = run_cli(self, ["host", "list"], cwd=ROOT)
        payload = json.loads(proc.stdout)
        self.assertTrue(isinstance(payload, list))
        names = {entry["name"] for entry in payload}
        self.assertIn("version", names)
        self.assertIn("print", names)
        self.assertEqual([entry["name"] for entry in payload], sorted(names))

        version_entry = next(entry for entry in payload if entry["name"] == "version")
        self.assertEqual(version_entry["arity"], 0)
        self.assertFalse(version_entry["side_effecting"])
        self.assertEqual(version_entry["arg_kinds"], [])
        self.assertEqual(version_entry["return_kind"], "int")

        print_entry = next(entry for entry in payload if entry["name"] == "print")
        self.assertEqual(print_entry["arg_kinds"], ["value"])
        self.assertEqual(print_entry["return_kind"], "int")

        parse_entry = next(entry for entry in payload if entry["name"] == "int.parse")
        self.assertEqual(parse_entry["arg_kinds"], ["string"])
        self.assertEqual(parse_entry["return_kind"], "int")

        safe_proc = run_cli(self, ["host", "list", "--safe-only"], cwd=ROOT)
        safe_payload = json.loads(safe_proc.stdout)
        self.assertTrue(isinstance(safe_payload, list))
        self.assertTrue(all(not entry["side_effecting"] for entry in safe_payload))
        safe_names = {entry["name"] for entry in safe_payload}
        self.assertIn("version", safe_names)
        self.assertIn("int.parse", safe_names)
        self.assertNotIn("print", safe_names)
        self.assertNotIn("read", safe_names)

        compact_proc = run_cli(self, ["host", "list", "--compact"], cwd=ROOT)
        self.assertEqual(payload, json.loads(compact_proc.stdout))

    def test_host_describe_command(self) -> None:
        proc = run_cli(self, ["host", "describe"], cwd=ROOT)
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
        self.assertIn("int.parse", names)
        self.assertIsInstance(payload["capabilities_signature"], str)
        self.assertEqual(
            payload["capabilities_signature"],
            hashlib.sha256(json.dumps(caps, sort_keys=True).encode("utf-8")).hexdigest(),
        )

        safe_proc = run_cli(self, ["host", "describe", "--safe-only"], cwd=ROOT)
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

        second_proc = run_cli(self, ["host", "describe"], cwd=ROOT)
        second_payload = json.loads(second_proc.stdout)
        self.assertEqual(
            second_payload["capabilities_signature"],
            payload["capabilities_signature"],
        )

        compact_proc = run_cli(self, ["host", "describe", "--compact"], cwd=ROOT)
        self.assertEqual(payload, json.loads(compact_proc.stdout))

    def test_string_program_cli_roundtrip_and_disasm(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "string.ax"
            bc = Path(td) / "string.axb"
            src.write_text('print "hello, axiom"\n', encoding="utf-8")

            interp_proc = run_cli(self, ["interp", str(src)], cwd=ROOT)
            self.assertEqual(interp_proc.stdout, "hello, axiom\n")

            run_cli(self, ["compile", str(src), "-o", str(bc)], cwd=ROOT)
            vm_proc = run_cli(self, ["vm", str(bc)], cwd=ROOT)
            self.assertEqual(vm_proc.stdout, "hello, axiom\n")

            run_proc = run_cli(self, ["run", str(src)], cwd=ROOT)
            self.assertEqual(run_proc.stdout, "hello, axiom\n")

            disasm_proc = run_cli(self, ["disasm", str(bc)], cwd=ROOT)
            self.assertIn("CONST_STRING", disasm_proc.stdout)
            self.assertIn("'hello, axiom'", disasm_proc.stdout)

    def test_compile_reports_json_ok(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "string.ax"
            bc = Path(td) / "string.axb"
            src.write_text('print "hello, axiom"\n', encoding="utf-8")
            payload = run_cli_json(
                self,
                ["compile", str(src), "-o", str(bc), "--json"],
                cwd=ROOT,
            )
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["command"], "compile")
        self.assertEqual(payload["file"], str(src))
        self.assertEqual(payload["output"], str(bc))
        self.assertGreater(payload["bytes"], 0)
        self.assertEqual(payload["bytecode"]["version_minor"], 11)
        self.assertGreater(payload["bytecode"]["instruction_count"], 0)

    def test_compile_reports_json_error(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "bad.ax"
            bc = Path(td) / "bad.axb"
            src.write_text("return 1\n", encoding="utf-8")
            payload = run_cli_json(
                self,
                ["compile", str(src), "-o", str(bc), "--json"],
                cwd=ROOT,
                expect_code=1,
            )
        self.assertFalse(payload["ok"])
        self.assertEqual(payload["command"], "compile")
        self.assertEqual(payload["file"], str(src))
        self.assertEqual(payload["output"], str(bc))
        self.assertEqual(payload["error"]["kind"], "AxiomParseError")
        self.assertEqual(
            Path(payload["error"]["location"]["path"]).resolve(),
            src.resolve(),
        )
        self.assertIn("return outside function", payload["error"]["message"])

    def test_bool_program_cli_roundtrip_and_disasm(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "bool.ax"
            bc = Path(td) / "bool.axb"
            src.write_text("let ready: bool = true\nprint ready\n", encoding="utf-8")

            interp_proc = run_cli(self, ["interp", str(src)], cwd=ROOT)
            self.assertEqual(interp_proc.stdout, "true\n")

            run_cli(self, ["compile", str(src), "-o", str(bc)], cwd=ROOT)
            vm_proc = run_cli(self, ["vm", str(bc)], cwd=ROOT)
            self.assertEqual(vm_proc.stdout, "true\n")

            run_proc = run_cli(self, ["run", str(src)], cwd=ROOT)
            self.assertEqual(run_proc.stdout, "true\n")

            disasm_proc = run_cli(self, ["disasm", str(bc)], cwd=ROOT)
            self.assertIn("CONST_BOOL true", disasm_proc.stdout)

    def test_imported_modules_execute(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            mod = Path(td) / "math_module.ax"
            mod.write_text("fn add(a: int, b: int): int {\n  return a + b\n}\n", encoding="utf-8")

            main = Path(td) / "main.ax"
            main.write_text('import "math_module"\nprint math_module.add(6, 7)\n', encoding="utf-8")

            proc = run_cli(self, ["interp", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "13\n")
            proc = run_cli(self, ["run", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "13\n")

    def test_imported_modules_execute_with_alias(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            mod = Path(td) / "math_module.ax"
            mod.write_text("fn add(a: int, b: int): int {\n  return a + b\n}\n", encoding="utf-8")

            main = Path(td) / "main.ax"
            main.write_text(
                'import "math_module" as MATH\nprint MATH.add(9, 8)\n',
                encoding="utf-8",
            )

            proc = run_cli(self, ["interp", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "17\n")
            proc = run_cli(self, ["run", str(main)], cwd=ROOT)
            self.assertEqual(proc.stdout, "17\n")


if __name__ == "__main__":
    unittest.main()
