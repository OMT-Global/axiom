import io
from pathlib import Path
import unittest

from axiom.api import compile_to_bytecode, parse_program
from axiom.errors import (
    AxiomCompileError,
    AxiomParseError,
    AxiomRuntimeError,
    MultiAxiomError,
    Span,
)
from axiom.host import register_host_builtin, reset_host_builtins, unregister_host_builtin
from axiom.interpreter import Interpreter
from axiom.vm import Vm
from tests.helpers import assert_program_parity


class CoreErrorTests(unittest.TestCase):
    def test_assign_undefined_compile_error(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("x = 1\n")

    def test_interpreter_division_by_zero(self) -> None:
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(parse_program("1 / 0\n"), io.StringIO())

    def test_vm_division_by_zero(self) -> None:
        bc = compile_to_bytecode("1 / 0\n")
        with self.assertRaises(AxiomRuntimeError):
            Vm(locals_count=bc.locals_count).run(bc, io.StringIO())

    def test_interpreter_lexical_scope_shadow(self) -> None:
        src = """
let x: int = 1
{
  let x: int = 2
}
print x
"""
        out = io.StringIO()
        Interpreter().run(parse_program(src), out)
        self.assertEqual(out.getvalue(), "1\n")

    def test_return_outside_function_parse_error(self) -> None:
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("return 1\n")

    def test_parse_error_includes_path_and_location(self) -> None:
        src = "let x: int = 1\nreturn 1\n"
        with self.assertRaises(AxiomParseError) as cm:
            parse_program(src, path=Path("bad-program.ax"))
        msg = str(cm.exception)
        self.assertIn("bad-program.ax:2:1", msg)
        self.assertIn("return outside function", msg)
        self.assertIn("return 1", msg)
        self.assertIn("^", msg)

    def test_rendered_error_redacts_github_token_source_snippet(self) -> None:
        token = "gh" + "p_" + ("A" * 36)
        src = f'let token: string = "{token}"\n'
        err = AxiomParseError("bad token", Span(20, 25), src, "secrets.ax")

        msg = str(err)
        payload = err.to_dict()

        self.assertNotIn(token, msg)
        self.assertNotIn(token, str(payload))
        self.assertIn("[REDACTED_SECRET]", msg)
        self.assertIn("[REDACTED_SECRET]", str(payload))
        self.assertIn("secrets.ax:1:21", msg)

    def test_rendered_error_preserves_normal_source_snippet(self) -> None:
        src = "let count: int = true\n"
        err = AxiomParseError("expected int", Span(17, 21), src, "normal.ax")

        msg = str(err)

        self.assertIn("let count: int = true", msg)
        self.assertIn("normal.ax:1:18", msg)
        self.assertIn("^", msg)

    def test_compile_undefined_function(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("print unknown(1)\n")

    def test_compile_undefined_function_suggests_close_match(self) -> None:
        src = """
fn answer(value: int): int {
  return value + 1
}

print anser(41)
"""
        with self.assertRaises(AxiomCompileError) as cm:
            compile_to_bytecode(src)
        self.assertIn("did you mean 'answer'?", str(cm.exception))

    def test_parse_reserved_host_function_name(self) -> None:
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode(
                """
fn host(): int {
  return 1
}
"""
            )

    def test_parse_reserved_host_identifier(self) -> None:
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("let host: int = 1\n")
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("host = 1\n")
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode(
                """
fn f(host: int): int {
  return host
}
"""
            )

    def test_nested_function_definition_support(self) -> None:
        src = """
fn outer(): int {
  fn inner(): int {
    return 1
  }
  return inner() + 1
}

print outer()
"""
        assert_program_parity(self, src, "2\n", label="nested-function")

    def test_compile_arity_mismatch(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode(
                """
fn f(a: int, b: int): int {
  return a + b
}
print f(1)
"""
            )

    def test_compile_closure_undefined_capture(self) -> None:
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode(
                """
fn outer(): int {
  fn inner(): int {
    return missing + 1
  }
  return inner()
}
"""
            )

    def test_runtime_closure_capture_reads_writes(self) -> None:
        src = """
fn counter(): int {
  let value: int = 0
  fn inc(): int {
    value = value + 1
    return value
  }
  inc()
  inc()
  return value
}

print counter()
"""
        assert_program_parity(self, src, "2\n", label="closure-capture")

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

    def test_compile_rejects_mixed_addition_when_statically_known(self) -> None:
        with self.assertRaises(AxiomCompileError) as cm:
            compile_to_bytecode('print 1 + "x"\n')
        self.assertIn("matching int or string operands", str(cm.exception))

    def test_compile_rejects_string_comparison_when_statically_known(self) -> None:
        with self.assertRaises(AxiomCompileError) as cm:
            compile_to_bytecode('print "a" < "b"\n')
        self.assertIn("operator '<' expects int operands", str(cm.exception))

    def test_compile_rejects_string_condition_when_statically_known(self) -> None:
        with self.assertRaises(AxiomCompileError) as cm:
            compile_to_bytecode('if "ready" { print 1 }\n')
        self.assertIn("if condition expects bool, got string", str(cm.exception))

    def test_compile_rejects_host_parse_int_argument_type_mismatch(self) -> None:
        with self.assertRaises(AxiomCompileError) as cm:
            compile_to_bytecode("print host.int.parse(7)\n")
        self.assertIn("expects string, got int", str(cm.exception))

    def test_compile_custom_host_builtin(self) -> None:
        def double(args: list[int], _out) -> int:
            return args[0] * 2

        register_host_builtin("double", 1, False, double)
        try:
            out = io.StringIO()
            Interpreter().run(parse_program("print host.double(21)\n"), out)
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
            out = io.StringIO()
            Interpreter().run(parse_program("print host.triple(7)\n"), out)
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

    def test_compile_unknown_host_function_suggests_close_match(self) -> None:
        with self.assertRaises(AxiomCompileError) as cm:
            compile_to_bytecode("print host.abx(-5)\n")
        self.assertIn("did you mean 'abs'?", str(cm.exception))

    def test_compile_unknown_type_suggests_close_match(self) -> None:
        with self.assertRaises(AxiomParseError) as cm:
            compile_to_bytecode("let ready: boool = true\n")
        self.assertIn("did you mean 'bool'?", str(cm.exception))

    def test_compile_undefined_variable_suggests_close_match(self) -> None:
        src = """
fn main(): int {
  let answer: int = 41
  return asnwer + 1
}

print main()
"""
        with self.assertRaises(AxiomCompileError) as cm:
            compile_to_bytecode(src)
        self.assertIn("did you mean 'answer'?", str(cm.exception))

    def test_host_registry_duplicate_name(self) -> None:
        def noop(args: list[int], _out) -> int:
            return 0

        with self.assertRaises(ValueError):
            register_host_builtin("print", 0, False, noop)

    def test_multi_error_top_level_collects_all(self) -> None:
        # Two independent type errors at top level — should be collected together.
        src = """
let x: int = "hello"
let y: bool = 42
"""
        with self.assertRaises(MultiAxiomError) as cm:
            compile_to_bytecode(src)
        errors = cm.exception.errors
        self.assertEqual(len(errors), 2)
        messages = " ".join(str(e) for e in errors)
        self.assertIn("string", messages)
        self.assertIn("int", messages)

    def test_multi_error_function_body_collects_all(self) -> None:
        # Two independent type errors inside a function body.
        src = """
fn broken(): int {
  let x: int = "wrong"
  let y: bool = 99
  return 0
}
"""
        with self.assertRaises(MultiAxiomError) as cm:
            compile_to_bytecode(src)
        self.assertGreaterEqual(len(cm.exception.errors), 2)

    def test_multi_error_to_dict(self) -> None:
        src = """
let x: int = "a"
let y: bool = 1
"""
        with self.assertRaises(MultiAxiomError) as cm:
            compile_to_bytecode(src)
        d = cm.exception.to_dict()
        self.assertEqual(d["kind"], "MultiAxiomError")
        self.assertIsInstance(d["errors"], list)
        self.assertGreaterEqual(len(d["errors"]), 2)

    def test_single_error_still_raises_axiom_compile_error(self) -> None:
        # A single error should still raise AxiomCompileError, not MultiAxiomError.
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode('let x: int = "hello"\n')


if __name__ == "__main__":
    unittest.main()
