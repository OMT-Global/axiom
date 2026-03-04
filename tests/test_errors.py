import unittest
import io

from axiom.api import compile_to_bytecode
from axiom.api import parse_program
from axiom.errors import AxiomCompileError, AxiomParseError, AxiomRuntimeError
from axiom.interpreter import Interpreter
from axiom.vm import Vm


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

    def test_runtime_host_version(self) -> None:
        program = parse_program("print host.version()\n")
        out = io.StringIO()
        Interpreter().run(program, out)
        self.assertEqual(out.getvalue(), "4\n")

    def test_runtime_host_print_requires_explicit_allow(self) -> None:
        program = parse_program("host.print(1)\n")
        with self.assertRaises(AxiomRuntimeError):
            Interpreter().run(program, io.StringIO())


if __name__ == "__main__":
    unittest.main()
