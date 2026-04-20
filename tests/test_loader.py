import os
from pathlib import Path
import tempfile
import unittest

from axiom.ast import FunctionDefStmt
from axiom.errors import AxiomCompileError
from axiom.loader import ModuleLoader, load_program_file


class LoaderTests(unittest.TestCase):
    def test_loader_uses_module_search_paths(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            shared = root / "shared"
            shared.mkdir()
            shared.joinpath("math_module.ax").write_text(
                "fn add(a: int, b: int): int { return a + b }\n",
                encoding="utf-8",
            )
            app = root / "app"
            app.mkdir()
            main = app / "main.ax"
            main.write_text('import "math_module"\n', encoding="utf-8")

            program = load_program_file(main, module_search_paths=[shared])
            function_names = [
                stmt.name for stmt in program.stmts if isinstance(stmt, FunctionDefStmt)
            ]
            self.assertIn("math_module.add", function_names)

    def test_loader_attaches_import_note_for_module_validation_errors(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("bad.ax").write_text("print 9\n", encoding="utf-8")
            root.joinpath("main.ax").write_text('import "bad"\n', encoding="utf-8")

            with self.assertRaises(AxiomCompileError) as cm:
                ModuleLoader().load_file(root / "main.ax")

            error = cm.exception
            self.assertEqual(len(error.notes), 1)
            self.assertEqual(error.notes[0].message, "imported from here")
            self.assertTrue(str(error.notes[0].path).endswith("main.ax"))

    @unittest.skipUnless(hasattr(os, "symlink"), "symlink support is required")
    def test_loader_rejects_symlinked_import_outside_search_root(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            app = root / "app"
            app.mkdir()
            outside = root / "outside.ax"
            outside.write_text(
                "fn leaked(): int { return 7 }\n",
                encoding="utf-8",
            )
            os.symlink(outside, app / "escape.ax")
            main = app / "main.ax"
            main.write_text('import "escape"\n', encoding="utf-8")

            with self.assertRaises(AxiomCompileError) as cm:
                ModuleLoader().load_file(main)

            self.assertIn(
                "import path resolves outside module search root",
                cm.exception.message,
            )


if __name__ == "__main__":
    unittest.main()
