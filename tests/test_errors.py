import unittest
import io
from unittest.mock import patch
from pathlib import Path
import tempfile

from axiom.api import compile_to_bytecode, compile_file
from axiom.api import parse_program
from axiom.ast import (
    AssignStmt,
    BlockStmt,
    FunctionDefStmt,
    IntLit,
    LetStmt,
    Program,
    ReturnStmt,
    Span,
    VarRef,
)
from axiom.errors import AxiomCompileError, AxiomParseError, AxiomRuntimeError
from axiom.interpreter import Interpreter
from axiom.vm import Vm
from axiom.bytecode import Op, VERSION_MINOR
from axiom.host import register_host_builtin, reset_host_builtins, unregister_host_builtin


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

    def test_parse_error_includes_path_and_location(self) -> None:
        src = "let x = 1\nreturn 1\n"
        with self.assertRaises(AxiomParseError) as cm:
            parse_program(src, path=Path("bad-program.ax"))
        msg = str(cm.exception)
        self.assertIn("bad-program.ax:2:1", msg)
        self.assertIn("return outside function", msg)
        self.assertIn("return 1", msg)
        self.assertIn("^", msg)

    def test_compile_undefined_function(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("print unknown(1)\n")

    def test_parse_reserved_host_function_name(self) -> None:
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("""
fn host() {
  return 1
}
""")

    def test_parse_reserved_host_identifier(self) -> None:
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("let host = 1\n")
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("host = 1\n")
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("""
fn f(host) {
  return host
}
""")

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

    def test_compile_host_allowed_list_enforcement(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("host.abs(-5)\n", allowed_host_calls={"print"})

        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("host.abs(-5)\n", allowed_host_calls=set())

        bc = compile_to_bytecode("print host.abs(-5)\n", allowed_host_calls={"abs"})
        out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, out)
        self.assertEqual(out.getvalue(), "5\n")

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

    def test_unregister_custom_host_builtin(self) -> None:
        def triple(args: list[int], _out) -> int:
            return args[0] * 3

        register_host_builtin("triple", 1, False, triple)
        try:
            program = parse_program("print host.triple(7)\n")
            out = io.StringIO()
            Interpreter().run(program, out)
            self.assertEqual(out.getvalue(), "21\n")

            unregister_host_builtin("triple")
            with self.assertRaises(AxiomCompileError):
                compile_to_bytecode("print host.triple(7)\n")
        finally:
            reset_host_builtins()

    def test_unregister_default_host_builtin_not_allowed(self) -> None:
        with self.assertRaises(ValueError):
            unregister_host_builtin("print")

    def test_unregister_unknown_host_builtin(self) -> None:
        with self.assertRaises(KeyError):
            unregister_host_builtin("missing")

    def test_compile_unknown_host_function(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("host.unknown(1)\n")

    def test_compile_non_host_namespace_call(self) -> None:
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("foo.bar(1)\n")

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

    def test_compile_circular_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("a.ax").write_text('import "b"\n', encoding="utf-8")
            root.joinpath("b.ax").write_text('import "a"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError):
                compile_file(root.joinpath("a.ax"))

    def test_compile_rejects_absolute_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("main.ax").write_text('import "/etc/hosts"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError):
                compile_file(root.joinpath("main.ax"))

    def test_compile_rejects_parent_traversal_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("mod.ax").write_text('print 0\n', encoding="utf-8")
            root.joinpath("main.ax").write_text('import "../mod"\n', encoding="utf-8")
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

    def test_vm_host_call_by_name(self) -> None:
        bc = compile_to_bytecode("print host.abs(-12)\n")
        host_calls = [i for i in bc.instructions if i.op == Op.HOST_CALL]
        self.assertEqual(len(host_calls), 1)
        host_name_index = host_calls[0].arg
        self.assertIsNotNone(host_name_index)
        self.assertEqual(bc.strings[int(host_name_index)], "abs")

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

    def test_runtime_non_host_namespace_call(self) -> None:
        with self.assertRaises(AxiomParseError):
            parse_program("foo.bar(1)\n")

    def test_runtime_reserved_host_identifier_let(self) -> None:
        program = Program(
            [LetStmt(name="host", expr=IntLit(1, Span(0, 1)), span=Span(0, 1))]
        )
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    def test_runtime_reserved_host_identifier_param(self) -> None:
        program = Program(
            [
                FunctionDefStmt(
                    name="f",
                    params=["host"],
                    body=BlockStmt(
                        stmts=[
                            ReturnStmt(expr=VarRef(name="host", span=Span(0, 4)), span=Span(0, 4))
                        ],
                        span=Span(0, 6),
                    ),
                    span=Span(0, 8),
                )
            ]
        )
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    def test_runtime_reserved_host_identifier_assign(self) -> None:
        program = Program(
            [
                LetStmt(name="x", expr=IntLit(1, Span(0, 1)), span=Span(0, 1)),
                AssignStmt(name="host", expr=IntLit(2, Span(2, 3)), span=Span(2, 4)),
            ]
        )
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

    def test_runtime_reserved_host_function_name(self) -> None:
        program = Program(
            [
                FunctionDefStmt(
                    name="host",
                    params=[],
                    body=BlockStmt(
                        stmts=[ReturnStmt(expr=IntLit(0, Span(0, 1)), span=Span(0, 1))],
                        span=Span(0, 3),
                    ),
                    span=Span(0, 5),
                )
            ]
        )
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())

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
