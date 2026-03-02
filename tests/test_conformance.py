import io
import os
import unittest

from axiom.api import compile_to_bytecode, parse_program
from axiom.interpreter import Interpreter
from axiom.vm import Vm

ROOT = os.path.dirname(__file__)
PROGS = os.path.join(ROOT, "programs")


def load(name: str) -> tuple[str, str]:
    with open(os.path.join(PROGS, f"{name}.ax"), "r", encoding="utf-8") as f:
        src = f.read()
    with open(os.path.join(PROGS, f"{name}.out"), "r", encoding="utf-8") as f:
        expected = f.read()
    return src, expected


class ConformanceTests(unittest.TestCase):
    def _run_interpreter(self, src: str) -> str:
        program = parse_program(src)
        out = io.StringIO()
        interp = Interpreter()
        interp.run(program, out)
        return out.getvalue()

    def _run_vm(self, src: str) -> str:
        bc = compile_to_bytecode(src)
        out = io.StringIO()
        vm = Vm(locals_count=bc.locals_count)
        vm.run(bc, out)
        return out.getvalue()

    def test_programs_match_expected_and_each_other(self):
        for name in ["arith", "vars", "expr_stmt"]:
            src, expected = load(name)

            interp_out = self._run_interpreter(src)
            vm_out = self._run_vm(src)

            self.assertEqual(interp_out, expected, f"interpreter mismatch: {name}")
            self.assertEqual(vm_out, expected, f"vm mismatch: {name}")


if __name__ == "__main__":
    unittest.main()
