import importlib.util
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts/quality/crap_indicators.py"
SPEC = importlib.util.spec_from_file_location("crap_indicators", SCRIPT_PATH)
crap_indicators = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = crap_indicators
SPEC.loader.exec_module(crap_indicators)


class CrapIndicatorTests(unittest.TestCase):
    def test_python_complexity_counts_common_branching_constructs(self):
        module = ast_parse(
            """
            def classify(value):
                if value > 10 and value < 20:
                    return "teenish"
                for item in [value]:
                    if item:
                        return "truthy"
                return "other"
            """
        )

        function = module.body[0]

        self.assertEqual(crap_indicators.python_complexity(function), 5)

    def test_python_coverage_is_applied_to_function_span(self):
        indicator = crap_indicators.FunctionIndicator(
            language="python",
            path="sample.py",
            name="sample",
            start_line=10,
            end_line=14,
            complexity=4,
            coverage=None,
            crap=None,
            covered_lines=None,
            executable_lines=None,
        )

        crap_indicators.apply_line_coverage(
            indicator,
            executable={9, 10, 11, 12, 14, 20},
            executed={10, 12, 20},
        )

        self.assertEqual(indicator.covered_lines, 2)
        self.assertEqual(indicator.executable_lines, 4)
        self.assertAlmostEqual(indicator.coverage, 0.5)
        self.assertAlmostEqual(indicator.crap, 6.0)

    def test_rust_function_discovery_and_lcov_mapping(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "src/lib.rs"
            source.parent.mkdir()
            source.write_text(
                textwrap.dedent(
                    """
                    pub fn pick(value: i32) -> i32 {
                        if value > 0 {
                            value
                        } else {
                            0
                        }
                    }
                    """
                ).lstrip()
            )
            lcov = root / "coverage.lcov"
            lcov.write_text(
                "\n".join(
                    [
                        f"SF:{source}",
                        "DA:1,1",
                        "DA:2,1",
                        "DA:3,1",
                        "DA:5,0",
                        "end_of_record",
                    ]
                )
            )

            cwd = Path.cwd()
            try:
                import os

                os.chdir(root)
                coverage = crap_indicators.load_lcov(lcov, root)
                functions = crap_indicators.discover_rust(Path("src"), coverage)
            finally:
                os.chdir(cwd)

        self.assertEqual(len(functions), 1)
        self.assertEqual(functions[0].name, "pick")
        self.assertEqual(functions[0].complexity, 2)
        self.assertAlmostEqual(functions[0].coverage, 0.75)

    def test_rust_function_discovery_ignores_braces_inside_strings(self):
        functions = list(
            crap_indicators.rust_functions(
                [
                    "fn render() {",
                    '    out.push_str("if value { still a string }");',
                    "}",
                    "fn next() {",
                    "}",
                ]
            )
        )

        self.assertEqual([(name, start, end) for name, start, end, _ in functions], [
            ("render", 1, 3),
            ("next", 4, 5),
        ])


def ast_parse(source):
    import ast

    return ast.parse(textwrap.dedent(source))


if __name__ == "__main__":
    unittest.main()
