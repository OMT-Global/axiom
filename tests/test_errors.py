import unittest

from axiom.api import compile_to_bytecode
from axiom.errors import AxiomCompileError


class ErrorTests(unittest.TestCase):
    def test_assign_undefined_compile_error(self):
        with self.assertRaises(AxiomCompileError):
            compile_to_bytecode("x = 1\n")


if __name__ == "__main__":
    unittest.main()
