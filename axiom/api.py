from __future__ import annotations

from pathlib import Path
from typing import Set

from .lexer import Lexer
from .parser import Parser
from .ast import ImportStmt, Program
from .compiler import Compiler
from .bytecode import Bytecode
from .errors import AxiomCompileError


def parse_file(path: Path) -> Program:
    return _load_program_file(Path(path).resolve(), set(), set())


def parse_program(src: str) -> Program:
    toks = Lexer(src).tokenize()
    return Parser(toks).parse_program()


def compile_to_bytecode(src: str, *, allow_host_side_effects: bool = False) -> Bytecode:
    program = parse_program(src)
    return Compiler(allow_host_side_effects=allow_host_side_effects).compile(program)


def compile_file(path: Path, *, allow_host_side_effects: bool = False) -> Bytecode:
    return Compiler(allow_host_side_effects=allow_host_side_effects).compile(
        parse_file(path)
    )


def _load_program_file(path: Path, seen: Set[Path], loading: Set[Path]) -> Program:
    if path in seen:
        return Program(stmts=[])

    if path in loading:
        raise AxiomCompileError(f"circular import of {path}")

    if not path.exists():
        raise AxiomCompileError(f"cannot resolve import file {path}")

    loading.add(path)
    src = path.read_text(encoding="utf-8")
    program = parse_program(src)

    stmts = []
    for stmt in program.stmts:
        if isinstance(stmt, ImportStmt):
            import_path = _resolve_import_path(stmt.path, path)
            if not import_path.exists():
                raise AxiomCompileError(
                    f"cannot resolve import file {import_path}", stmt.span
                )
            stmts.extend(_load_program_file(import_path, seen, loading).stmts)
        else:
            stmts.append(stmt)

    loading.remove(path)
    seen.add(path)
    return Program(stmts=stmts)


def _resolve_import_path(raw: str, base_path: Path) -> Path:
    candidate = Path(raw)
    if candidate.suffix == "":
        candidate = candidate.with_suffix(".ax")
    if candidate.suffix not in (".ax", ".AX"):
        # preserve prior behavior for module names that may include dots
        candidate = Path(str(candidate) + ".ax")
    if not candidate.is_absolute():
        candidate = base_path.parent / candidate
    return candidate
