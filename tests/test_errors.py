import unittest
import io
from unittest.mock import patch
from pathlib import Path
import tempfile

from axiom.api import compile_to_bytecode, compile_file
from axiom.api import parse_program
from axiom.errors import AxiomCompileError, AxiomParseError, AxiomRuntimeError
from axiom.interpreter import Interpreter
from axiom.vm import Vm
from axiom.bytecode import VERSION_MINOR
from axiom.host import register_host_builtin, reset_host_builtins


class ErrorTests(unittest.TestCase):
    def test_assign_undefined_compile_error(self):
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("x = 1\n")

    def test_interpreter_division_by_zero(self) -> None:
        program = parse_program("1 / 0\n")
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    def test_vm_division_by_zero(self) -> None:
        bc = compile_to_bytecode("1 / 0\n")
        with self.assertRaises(AxiomRuntimeError):
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

    def test_interpreter_lexical_scope_shadow(self) -> None:
        program = parse_program("""
let x = 1
{
  let x = 2
}
print x
""")
        out = io.StringIO()
        Interpreter().run(program, out)
        self.assertEqual(out.getvalue(), "1\n")

    def test_return_outside_function_parse_error(self) -> None:
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("return 1\n")

    def test_compile_undefined_function(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("print unknown(1)\n")

    def test_compile_arity_mismatch(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("""
fn f(a, b) {
  return a + b
}
print f(1)
""")

    def test_compile_host_side_effect_blocked(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("host.print(1)\n")

    def test_compile_host_side_effect_allowed(self) -> None:
        compile_to_bytecode("host.print(1)\n", allow_host_side_effects=True)

    def test_compile_host_arity_mismatch(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("host.abs(1, 2)\n")
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("host.math.abs()\n")

    def test_compile_custom_host_builtin(self) -> None:
        def double(args: list[int], _out) -> int:
            return args[0] * 2

        register_host_builtin("double", 1, False, double)
        try:
            program = parse_program("print host.double(21)\n")
            out = io.StringIO()
            Interpreter().run(program, out)
            self.assertEqual(out.getvalue(), "42\n")

            bc = compile_to_bytecode("print host.double(21)\n")
            vm_out = io.StringIO()
            Vm(locals_count=bc.locals_count).run(bc, vm_out)
            self.assertEqual(vm_out.getvalue(), "42\n")
        finally:
            reset_host_builtins()

    def test_host_registry_duplicate_name(self) -> None:
        def noop(args: list[int], _out) -> int:
            return 0

        with self.assertRaises(ValueError):
            register_host_builtin("print", 0, False, noop)

    def test_compile_missing_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("main.ax").write_text('import "missing.ax"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError):
                compile_file(root.joinpath("main.ax"))

    def test_runtime_host_version(self) -> None:
        program = parse_program("print host.version()\n")
        out = io.StringIO()
        Interpreter().run(program, out)
        self.assertEqual(out.getvalue(), f"{VERSION_MINOR}\n")

    def test_runtime_host_abs(self) -> None:
        program = parse_program("print host.abs(-12)\n")
        out = io.StringIO()
        Interpreter().run(program, out)
        self.assertEqual(out.getvalue(), "12\n")

    def test_vm_host_version(self) -> None:
        bc = compile_to_bytecode("print host.version()\n")
        out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, out)
        self.assertEqual(out.getvalue(), f"{VERSION_MINOR}\n")

    def test_vm_host_abs(self) -> None:
        bc = compile_to_bytecode("print host.abs(-12)\n")
        out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, out)
        self.assertEqual(out.getvalue(), "12\n")

    def test_runtime_host_print_requires_explicit_allow(self) -> None:
        program = parse_program("host.print(1)\n")
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    def test_runtime_host_print_requires_explicit_allow_vm(self) -> None:
        bc = compile_to_bytecode("host.print(1)\n", allow_host_side_effects=True)
        with self.assertRaises(AxiomRuntimeError):
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

    def test_runtime_host_print_with_allow(self) -> None:
        program = parse_program("host.print(1)\n")
        out = io.StringIO()
        Interpreter(allow_host_side_effects=True).run(program, out)
        self.assertEqual(out.getvalue(), "1\n")

    def test_runtime_host_print_with_allow_vm(self) -> None:
        bc = compile_to_bytecode("host.print(1)\n", allow_host_side_effects=True)
        out = io.StringIO()
        Vm(locals_count=bc.locals_count, allow_host_side_effects=True).run(bc, out)
        self.assertEqual(out.getvalue(), "1\n")

    @patch("builtins.input", return_value="41")
    def test_runtime_host_read_with_allow(self, fake_input) -> None:
        program = parse_program("print host.read(123)\n")
        out = io.StringIO()
        Interpreter(allow_host_side_effects=True).run(program, out)
        self.assertEqual(out.getvalue(), "41\n")
        fake_input.assert_called_once_with("123")

    @patch("builtins.input", return_value="41")
    def test_runtime_host_read_with_allow_vm(self, fake_input) -> None:
        bc = compile_to_bytecode("print host.read(123)\n", allow_host_side_effects=True)
        out = io.StringIO()
        Vm(locals_count=bc.locals_count, allow_host_side_effects=True).run(bc, out)
        self.assertEqual(out.getvalue(), "41\n")
        fake_input.assert_called_once_with("123")


if __name__ == "__main__":
    unittest.main()
