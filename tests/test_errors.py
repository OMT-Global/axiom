import unittest
import io

from axiom.api import compile_to_bytecode
from axiom.api import parse_program
from axiom.errors import AxiomCompileError, AxiomRuntimeError
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


if __name__ == "__main__":
    unittest.main()
