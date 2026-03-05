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
from axiom.host import host_contract_metadata, register_host_builtin, reset_host_builtins, unregister_host_builtin


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

    def test_nested_function_definition_support(self) -> None:
        program = parse_program(
            """
fn outer() {
  fn inner() {
    return 1
  }
  return inner() + 1
}

print outer()
"""
        )
        interp_out = io.StringIO()
        Interpreter().run(program, interp_out)
        self.assertEqual(interp_out.getvalue(), "2\n")

        bc = compile_to_bytecode(
            """
fn outer() {
  fn inner() {
    return 1
  }
  return inner() + 1
}

print outer()
"""
        )
        vm_out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, vm_out)
        self.assertEqual(vm_out.getvalue(), "2\n")

    def test_compile_arity_mismatch(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("""
fn f(a, b) {
  return a + b
}
print f(1)
""")

    def test_compile_closure_undefined_capture(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("""
fn outer() {
  fn inner() {
    return missing + 1
  }
  return inner()
}
""")

    def test_runtime_closure_capture_reads_writes(self) -> None:
        program = parse_program(
            """
fn counter() {
  let value = 0
  fn inc() {
    value = value + 1
    return value
  }
  inc()
  inc()
  return value
}

print counter()
"""
        )
        out = io.StringIO()
        Interpreter().run(program, out)
        self.assertEqual(out.getvalue(), "2\n")
        bc = compile_to_bytecode(
            """
fn counter() {
  let value = 0
  fn inc() {
    value = value + 1
    return value
  }
  inc()
  inc()
  return value
}

print counter()
"""
        )
        vm_out = io.StringIO()
        Vm(locals_count=bc.locals_count).run(bc, vm_out)
        self.assertEqual(vm_out.getvalue(), "2\n")

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

    def test_compile_import_alias(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text(
                "fn add(a, b) { return a + b }\n", encoding="utf-8"
            )
            root.joinpath("main.ax").write_text(
                'import "math_module" as math\nprint math.add(11, 9)\n',
                encoding="utf-8",
            )
            bc = compile_file(root.joinpath("main.ax"))
            out = io.StringIO()
            Vm(locals_count=bc.locals_count).run(bc, out)
            self.assertEqual(out.getvalue(), "20\n")

    def test_compile_import_dotted_alias(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text(
                "fn add(a, b) { return a + b }\n", encoding="utf-8"
            )
            root.joinpath("main.ax").write_text(
                'import "math_module" as tools.math\nprint tools.math.add(4, 5)\n',
                encoding="utf-8",
            )
            bc = compile_file(root.joinpath("main.ax"))
            out = io.StringIO()
            Vm(locals_count=bc.locals_count).run(bc, out)
            self.assertEqual(out.getvalue(), "9\n")

    def test_compile_imported_module_top_level_statement_disallowed(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text("print 9\n", encoding="utf-8")
            root.joinpath("main.ax").write_text('import "math_module"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError) as cm:
                compile_file(root.joinpath("main.ax"))
            msg = str(cm.exception)
            self.assertIn("imported module", msg)
            self.assertIn("print 9", msg)
            self.assertIn("math_module.ax:1", msg)
            self.assertIn("^", msg)

    def test_compile_import_duplicate_namespace(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text(
                "fn add(a, b) { return a + b }\n", encoding="utf-8"
            )
            root.joinpath("other_module.ax").write_text(
                "fn sub(a, b) { return a - b }\n", encoding="utf-8"
            )
            root.joinpath("main.ax").write_text(
                'import "math_module" as shared\nimport "other_module" as shared\n',
                encoding="utf-8",
            )
            with self.assertRaises(AxiomParseError):
                compile_file(root.joinpath("main.ax"))

    def test_compile_import_duplicate_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text(
                "fn add(a, b) { return a + b }\n", encoding="utf-8"
            )
            root.joinpath("main.ax").write_text(
                'import "math_module" as one\nimport "math_module" as two\n',
                encoding="utf-8",
            )
            with self.assertRaises(AxiomParseError):
                compile_file(root.joinpath("main.ax"))

    def test_compile_import_nested_default_namespace(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            modules = root / "math"
            modules.mkdir()
            modules.joinpath("math_utils.ax").write_text(
                "fn add(a, b) { return a + b }\n", encoding="utf-8"
            )
            root.joinpath("main.ax").write_text(
                'import "math/math_utils"\nprint math.math_utils.add(4, 5)\n', encoding="utf-8"
            )
            bc = compile_file(root.joinpath("main.ax"))
            out = io.StringIO()

            Vm(locals_count=bc.locals_count).run(bc, out)
            self.assertEqual(out.getvalue(), "9\n")

    def test_compile_import_transitive_namespace_collision(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("shared_module.ax").write_text(
                "fn add(a, b) { return a + b }\n", encoding="utf-8"
            )
            root.joinpath("inner_module.ax").write_text(
                'import "shared_module" as shared\nfn square(a) { return shared.add(a, a) }\n',
                encoding="utf-8",
            )
            root.joinpath("outer_module.ax").write_text(
                'import "shared_module" as shared\nfn add(a, b) { return a + b }\n',
                encoding="utf-8",
            )
            root.joinpath("main.ax").write_text(
                'import "outer_module"\nimport "inner_module"\n',
                encoding="utf-8",
            )
            with self.assertRaises(AxiomCompileError):
                compile_file(root.joinpath("main.ax"))

    def test_compile_import_alias_host_reserved(self) -> None:
        with self.assertRaises(AxiomParseError) as cm:
            parse_program('import "host/foo"\n')
        self.assertIn("import namespace cannot be 'host'", str(cm.exception))

        with self.assertRaises(AxiomParseError) as cm:
            compile_to_bytecode('import "math_module" as host.tools\n')
        self.assertIn("import namespace cannot be 'host'", str(cm.exception))

    def test_host_registry_duplicate_name(self) -> None:
        def noop(args: list[int], _out) -> int:
            return 0

        with self.assertRaises(ValueError):
            register_host_builtin("print", 0, False, noop)

    def test_compile_missing_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("main.ax").write_text('import "missing.ax"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError) as cm:
                compile_file(root.joinpath("main.ax"))
            msg = str(cm.exception)
            self.assertIn("cannot resolve import file", msg)
            self.assertIn("main.ax:1", msg)
            self.assertIn("import \"missing.ax\"", msg)
            self.assertIn("^", msg)

    def test_compile_circular_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("a.ax").write_text('import "b"\n', encoding="utf-8")
            root.joinpath("b.ax").write_text('import "a"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError) as cm:
                compile_file(root.joinpath("a.ax"))
            msg = str(cm.exception)
            self.assertIn("circular import", msg)
            self.assertIn("b.ax:1", msg)
            self.assertIn("import \"a\"", msg)
            self.assertIn("^", msg)

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

    def test_host_contract_signature_tracks_capability_state(self) -> None:
        base = host_contract_metadata()
        base_signature = base["capabilities_signature"]

        def probe(_args: list[int], _out) -> int:
            return 7

        register_host_builtin("sig_probe", 0, False, probe)
        try:
            with_probe = host_contract_metadata()
            self.assertNotEqual(base_signature, with_probe["capabilities_signature"])
            self.assertIn("capabilities_signature", with_probe)
            self.assertEqual(with_probe["schema_version"], 1)
            self.assertIn("sig_probe", {e["name"] for e in with_probe["capabilities"]})
        finally:
            reset_host_builtins()


if __name__ == "__main__":
    unittest.main()
