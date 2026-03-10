import io
from pathlib import Path
import tempfile
import unittest

from axiom.api import compile_file, compile_to_bytecode, parse_program
from axiom.errors import AxiomCompileError, AxiomParseError
from axiom.vm import Vm


class ImportErrorTests(unittest.TestCase):
    def test_compile_non_host_namespace_call(self) -> None:
        with self.assertRaises(AxiomParseError):
            compile_to_bytecode("foo.bar(1)\n")

    def test_compile_import_alias(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text(
                "fn add(a: int, b: int): int { return a + b }\n",
                encoding="utf-8",
            )
            root.joinpath("main.ax").write_text(
                'import "math_module" as math\nprint math.add(11, 9)\n',
                encoding="utf-8",
            )
            bc = compile_file(root / "main.ax")
            out = io.StringIO()
            Vm(locals_count=bc.locals_count).run(bc, out)
            self.assertEqual(out.getvalue(), "20\n")

    def test_compile_import_dotted_alias(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text(
                "fn add(a: int, b: int): int { return a + b }\n",
                encoding="utf-8",
            )
            root.joinpath("main.ax").write_text(
                'import "math_module" as tools.math\nprint tools.math.add(4, 5)\n',
                encoding="utf-8",
            )
            bc = compile_file(root / "main.ax")
            out = io.StringIO()
            Vm(locals_count=bc.locals_count).run(bc, out)
            self.assertEqual(out.getvalue(), "9\n")

    def test_compile_imported_module_top_level_statement_disallowed(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text("print 9\n", encoding="utf-8")
            root.joinpath("main.ax").write_text('import "math_module"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError) as cm:
                compile_file(root / "main.ax")
            msg = str(cm.exception)
            self.assertIn("imported module", msg)
            self.assertIn("print 9", msg)
            self.assertIn("math_module.ax:1", msg)
            self.assertIn("note: imported from here", msg)
            self.assertIn("main.ax:1", msg)
            self.assertIn('import "math_module"', msg)
            self.assertIn("^", msg)

    def test_compile_import_duplicate_namespace(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text(
                "fn add(a: int, b: int): int { return a + b }\n",
                encoding="utf-8",
            )
            root.joinpath("other_module.ax").write_text(
                "fn sub(a: int, b: int): int { return a - b }\n",
                encoding="utf-8",
            )
            root.joinpath("main.ax").write_text(
                'import "math_module" as shared\nimport "other_module" as shared\n',
                encoding="utf-8",
            )
            with self.assertRaises(AxiomParseError):
                compile_file(root / "main.ax")

    def test_compile_import_duplicate_path(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("math_module.ax").write_text(
                "fn add(a: int, b: int): int { return a + b }\n",
                encoding="utf-8",
            )
            root.joinpath("main.ax").write_text(
                'import "math_module" as one\nimport "math_module" as two\n',
                encoding="utf-8",
            )
            with self.assertRaises(AxiomParseError):
                compile_file(root / "main.ax")

    def test_compile_import_nested_default_namespace(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            modules = root / "math"
            modules.mkdir()
            modules.joinpath("math_utils.ax").write_text(
                "fn add(a: int, b: int): int { return a + b }\n",
                encoding="utf-8",
            )
            root.joinpath("main.ax").write_text(
                'import "math/math_utils"\nprint math.math_utils.add(4, 5)\n',
                encoding="utf-8",
            )
            bc = compile_file(root / "main.ax")
            out = io.StringIO()
            Vm(locals_count=bc.locals_count).run(bc, out)
            self.assertEqual(out.getvalue(), "9\n")

    def test_compile_import_transitive_namespace_collision(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("shared_module.ax").write_text(
                "fn add(a: int, b: int): int { return a + b }\n",
                encoding="utf-8",
            )
            root.joinpath("inner_module.ax").write_text(
                'import "shared_module" as shared\nfn square(a: int): int { return shared.add(a, a) }\n',
                encoding="utf-8",
            )
            root.joinpath("outer_module.ax").write_text(
                'import "shared_module" as shared\nfn add(a: int, b: int): int { return a + b }\n',
                encoding="utf-8",
            )
            root.joinpath("main.ax").write_text(
                'import "outer_module"\nimport "inner_module"\n',
                encoding="utf-8",
            )
            with self.assertRaises(AxiomCompileError):
                compile_file(root / "main.ax")

    def test_compile_import_alias_host_reserved(self) -> None:
        with self.assertRaises(AxiomParseError) as cm:
            parse_program('import "host/foo"\n')
        self.assertIn("import namespace cannot be 'host'", str(cm.exception))

        with self.assertRaises(AxiomParseError) as cm:
            compile_to_bytecode('import "math_module" as host.tools\n')
        self.assertIn("import namespace cannot be 'host'", str(cm.exception))

    def test_compile_missing_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("main.ax").write_text('import "missing.ax"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError) as cm:
                compile_file(root / "main.ax")
            msg = str(cm.exception)
            self.assertIn("cannot resolve import file", msg)
            self.assertIn("main.ax:1", msg)
            self.assertIn('import "missing.ax"', msg)
            self.assertIn("^", msg)

    def test_compile_circular_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("a.ax").write_text('import "b"\n', encoding="utf-8")
            root.joinpath("b.ax").write_text('import "a"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError) as cm:
                compile_file(root / "a.ax")
            msg = str(cm.exception)
            self.assertIn("circular import", msg)
            self.assertIn("b.ax:1", msg)
            self.assertIn('import "a"', msg)
            self.assertIn("^", msg)

    def test_compile_transitive_import_parse_error_includes_full_trace(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("bad.ax").write_text("return 1\n", encoding="utf-8")
            root.joinpath("middle.ax").write_text(
                'import "bad"\nfn ok(): int { return 1 }\n',
                encoding="utf-8",
            )
            root.joinpath("main.ax").write_text('import "middle"\n', encoding="utf-8")
            with self.assertRaises(AxiomParseError) as cm:
                compile_file(root / "main.ax")
            msg = str(cm.exception)
            self.assertIn("return outside function", msg)
            self.assertIn("bad.ax:1:1", msg)
            self.assertIn("middle.ax:1:1", msg)
            self.assertIn("main.ax:1:1", msg)
            self.assertIn('import "bad"', msg)
            self.assertIn('import "middle"', msg)
            self.assertEqual(msg.count("note: imported from here"), 2)

    def test_compile_rejects_absolute_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("main.ax").write_text('import "/etc/hosts"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError):
                compile_file(root / "main.ax")

    def test_compile_rejects_parent_traversal_import(self) -> None:
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            root.joinpath("mod.ax").write_text('print 0\n', encoding="utf-8")
            root.joinpath("main.ax").write_text('import "../mod"\n', encoding="utf-8")
            with self.assertRaises(AxiomCompileError):
                compile_file(root / "main.ax")


if __name__ == "__main__":
    unittest.main()
