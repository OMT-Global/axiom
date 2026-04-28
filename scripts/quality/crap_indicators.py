#!/usr/bin/env python3
"""Compute CRAP indicators for the Python and Rust implementation tracks."""

from __future__ import annotations

import argparse
import ast
import json
import math
import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


RUST_DECISION_TOKENS = (
    r"\belse\s+if\b",
    r"\bif\b",
    r"\bmatch\b",
    r"\bwhile\b",
    r"\bfor\b",
    r"&&",
    r"\|\|",
    r"\?",
    r"=>",
)


@dataclass
class FunctionIndicator:
    language: str
    path: str
    name: str
    start_line: int
    end_line: int
    complexity: int
    coverage: float | None
    crap: float | None
    covered_lines: int | None
    executable_lines: int | None


class PythonComplexityVisitor(ast.NodeVisitor):
    def __init__(self) -> None:
        self.functions: list[FunctionIndicator] = []
        self._stack: list[str] = []

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self._visit_function(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self._visit_function(node)

    def _visit_function(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        name = ".".join([*self._stack, node.name]) if self._stack else node.name
        self.functions.append(
            FunctionIndicator(
                language="python",
                path="",
                name=name,
                start_line=node.lineno,
                end_line=getattr(node, "end_lineno", node.lineno),
                complexity=python_complexity(node),
                coverage=None,
                crap=None,
                covered_lines=None,
                executable_lines=None,
            )
        )
        self._stack.append(node.name)
        self.generic_visit(node)
        self._stack.pop()


def python_complexity(node: ast.AST) -> int:
    complexity = 1
    for child in ast.walk(node):
        if isinstance(
            child,
            (
                ast.If,
                ast.For,
                ast.AsyncFor,
                ast.While,
                ast.IfExp,
                ast.ExceptHandler,
                ast.Assert,
            ),
        ):
            complexity += 1
        elif isinstance(child, ast.BoolOp):
            complexity += max(1, len(child.values) - 1)
        elif isinstance(child, ast.Match):
            complexity += max(1, len(child.cases))
        elif isinstance(child, ast.comprehension):
            complexity += 1 + len(child.ifs)
    return complexity


def load_coverage_py(path: Path | None, root: Path) -> dict[str, set[int]]:
    if path is None or not path.exists():
        return {}
    data = json.loads(path.read_text())
    covered: dict[str, set[int]] = {}
    for raw_file, file_data in data.get("files", {}).items():
        file_path = normalize_path(Path(raw_file), root)
        executed = set(file_data.get("executed_lines", []))
        missing = set(file_data.get("missing_lines", []))
        covered[str(file_path)] = executed | missing
        covered[f"{file_path}:executed"] = executed
    return covered


def load_lcov(path: Path | None, root: Path) -> dict[str, dict[int, int]]:
    if path is None or not path.exists():
        return {}
    current: Path | None = None
    lines: dict[str, dict[int, int]] = {}
    for raw in path.read_text().splitlines():
        if raw.startswith("SF:"):
            current = normalize_path(Path(raw[3:]), root)
            lines.setdefault(str(current), {})
        elif raw.startswith("DA:") and current is not None:
            line_raw, hits_raw, *_ = raw[3:].split(",")
            lines[str(current)][int(line_raw)] = int(hits_raw)
        elif raw == "end_of_record":
            current = None
    return lines


def normalize_path(path: Path, root: Path) -> Path:
    if path.is_absolute():
        try:
            return path.resolve().relative_to(root.resolve())
        except ValueError:
            return path.resolve()
    return path


def discover_python(root: Path, coverage: dict[str, set[int]]) -> list[FunctionIndicator]:
    functions: list[FunctionIndicator] = []
    for path in sorted(root.rglob("*.py")):
        source = path.read_text()
        tree = ast.parse(source, filename=str(path))
        visitor = PythonComplexityVisitor()
        visitor.visit(tree)
        rel = str(normalize_path(path, Path.cwd()))
        executable = coverage.get(rel)
        executed = coverage.get(f"{rel}:executed")
        for fn in visitor.functions:
            fn.path = rel
            apply_line_coverage(fn, executable, executed)
            functions.append(fn)
    return functions


def apply_line_coverage(
    indicator: FunctionIndicator, executable: set[int] | None, executed: set[int] | None
) -> None:
    if executable is None or executed is None:
        return
    fn_lines = {line for line in executable if indicator.start_line <= line <= indicator.end_line}
    if not fn_lines:
        return
    covered = fn_lines & executed
    indicator.covered_lines = len(covered)
    indicator.executable_lines = len(fn_lines)
    indicator.coverage = len(covered) / len(fn_lines)
    indicator.crap = crap_score(indicator.complexity, indicator.coverage)


def discover_rust(root: Path, coverage: dict[str, dict[int, int]]) -> list[FunctionIndicator]:
    functions: list[FunctionIndicator] = []
    for path in sorted(root.rglob("*.rs")):
        rel = str(normalize_path(path, Path.cwd()))
        lines = path.read_text().splitlines()
        for name, start, end, body in rust_functions(lines):
            indicator = FunctionIndicator(
                language="rust",
                path=rel,
                name=name,
                start_line=start,
                end_line=end,
                complexity=rust_complexity(body),
                coverage=None,
                crap=None,
                covered_lines=None,
                executable_lines=None,
            )
            apply_rust_coverage(indicator, coverage.get(rel))
            functions.append(indicator)
    return functions


def rust_functions(lines: list[str]) -> Iterable[tuple[str, int, int, str]]:
    fn_re = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>{}]*>)?\s*\(")
    i = 0
    while i < len(lines):
        match = fn_re.search(lines[i])
        if not match:
            i += 1
            continue
        start = i + 1
        brace_depth = 0
        seen_body = False
        body_lines: list[str] = []
        j = i
        while j < len(lines):
            line = strip_rust_comments_and_literals(lines[j])
            if "{" in line:
                seen_body = True
            if seen_body:
                body_lines.append(line)
            brace_depth += line.count("{") - line.count("}")
            if seen_body and brace_depth <= 0:
                break
            j += 1
        if seen_body:
            yield match.group(1), start, j + 1, "\n".join(body_lines)
            i = j + 1
        else:
            i += 1


def strip_rust_comments_and_literals(line: str) -> str:
    output: list[str] = []
    i = 0
    while i < len(line):
        if line.startswith("//", i):
            break
        if line[i] == "r" and i + 1 < len(line) and line[i + 1] in {'"', '#'}:
            i = skip_raw_string(line, i)
            output.append('""')
            continue
        if line[i] == '"':
            i = skip_quoted(line, i, '"')
            output.append('""')
            continue
        if line[i] == "'":
            i = skip_quoted(line, i, "'")
            output.append("''")
            continue
        output.append(line[i])
        i += 1
    return "".join(output)


def skip_quoted(line: str, start: int, quote: str) -> int:
    i = start + 1
    escaped = False
    while i < len(line):
        if escaped:
            escaped = False
        elif line[i] == "\\":
            escaped = True
        elif line[i] == quote:
            return i + 1
        i += 1
    return i


def skip_raw_string(line: str, start: int) -> int:
    i = start + 1
    hashes = 0
    while i < len(line) and line[i] == "#":
        hashes += 1
        i += 1
    if i >= len(line) or line[i] != '"':
        return start + 1
    terminator = '"' + ("#" * hashes)
    end = line.find(terminator, i + 1)
    return len(line) if end == -1 else end + len(terminator)


def rust_complexity(body: str) -> int:
    complexity = 1
    for token in RUST_DECISION_TOKENS:
        complexity += len(re.findall(token, body))
    return complexity


def apply_rust_coverage(indicator: FunctionIndicator, lines: dict[int, int] | None) -> None:
    if lines is None:
        return
    executable = {
        line_no for line_no in lines if indicator.start_line <= line_no <= indicator.end_line
    }
    if not executable:
        return
    covered = {line_no for line_no in executable if lines[line_no] > 0}
    indicator.covered_lines = len(covered)
    indicator.executable_lines = len(executable)
    indicator.coverage = len(covered) / len(executable)
    indicator.crap = crap_score(indicator.complexity, indicator.coverage)


def crap_score(complexity: int, coverage: float) -> float:
    return complexity**2 * math.pow(1 - coverage, 3) + complexity


def render_markdown(indicators: list[FunctionIndicator], limit: int) -> str:
    ranked = sorted(
        indicators,
        key=lambda item: (
            item.crap is None,
            -(item.crap or item.complexity),
            -item.complexity,
            item.path,
            item.start_line,
        ),
    )
    lines = [
        "# CRAP Indicators",
        "",
        "CRAP is `complexity^2 * (1 - coverage)^3 + complexity`.",
        "Rows without coverage still show complexity so uncovered setup is visible.",
        "",
        "| Rank | Lang | Path | Function | Lines | Complexity | Coverage | CRAP |",
        "| ---: | --- | --- | --- | ---: | ---: | ---: | ---: |",
    ]
    for rank, item in enumerate(ranked[:limit], start=1):
        coverage = "" if item.coverage is None else f"{item.coverage * 100:.1f}%"
        crap = "" if item.crap is None else f"{item.crap:.2f}"
        lines.append(
            "| "
            f"{rank} | {item.language} | `{item.path}` | `{item.name}` | "
            f"{item.start_line}-{item.end_line} | {item.complexity} | {coverage} | {crap} |"
        )
    lines.append("")
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python-root", type=Path, default=Path("axiom"))
    parser.add_argument("--rust-root", type=Path, default=Path("stage1/crates/axiomc/src"))
    parser.add_argument("--python-coverage", type=Path, default=Path(".quality/coverage/python.json"))
    parser.add_argument("--rust-lcov", type=Path, default=Path(".quality/coverage/rust.lcov"))
    parser.add_argument("--json-out", type=Path, default=Path(".quality/crap.json"))
    parser.add_argument("--markdown-out", type=Path, default=Path(".quality/crap.md"))
    parser.add_argument("--limit", type=int, default=40)
    parser.add_argument("--fail-on-crap-over", type=float, default=None)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = Path.cwd()
    python_coverage = load_coverage_py(args.python_coverage, root)
    rust_coverage = load_lcov(args.rust_lcov, root)
    indicators = [
        *discover_python(args.python_root, python_coverage),
        *discover_rust(args.rust_root, rust_coverage),
    ]

    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    args.markdown_out.parent.mkdir(parents=True, exist_ok=True)
    args.json_out.write_text(
        json.dumps([asdict(item) for item in indicators], indent=2, sort_keys=True) + "\n"
    )
    args.markdown_out.write_text(render_markdown(indicators, args.limit))

    print(f"Wrote {args.json_out}")
    print(f"Wrote {args.markdown_out}")

    if args.fail_on_crap_over is not None:
        offenders = [
            item
            for item in indicators
            if item.crap is not None and item.crap > args.fail_on_crap_over
        ]
        if offenders:
            print(f"{len(offenders)} functions exceed CRAP threshold {args.fail_on_crap_over}")
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
