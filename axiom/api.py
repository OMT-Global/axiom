from __future__ import annotations

from pathlib import Path
from typing import Optional, Set

from .lexer import Lexer
from .parser import Parser
from .ast import (
    Binary,
    BlockStmt,
    CallExpr,
    Expr,
    FunctionDefStmt,
    ImportStmt,
    IntLit,
    IfStmt,
    Program,
    ReturnStmt,
    UnaryNeg,
    VarRef,
    WhileStmt,
    AssignStmt,
    PrintStmt,
    LetStmt,
    ExprStmt,
    Stmt,
)
from .compiler import Compiler
from .bytecode import Bytecode
from .errors import AxiomCompileError


def parse_file(path: Path) -> Program:
    return _load_program_file(Path(path).resolve(), set(), set())


def parse_program(src: str, path: Path | str | None = None) -> Program:
    toks = Lexer(src, path=str(path) if path is not None else None).tokenize()
    return Parser(
        toks,
        source=src,
        source_path=str(path) if path is not None else None,
    ).parse_program()


def compile_to_bytecode(
    src: str,
    *,
    allow_host_side_effects: bool = False,
    allowed_host_calls: Optional[Set[str]] = None,
) -> Bytecode:
    program = parse_program(src, path=None)
    return Compiler(
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    ).compile(program)


def compile_file(
    path: Path,
    *,
    allow_host_side_effects: bool = False,
    allowed_host_calls: Optional[Set[str]] = None,
) -> Bytecode:
    return Compiler(
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    ).compile(parse_file(path))


def _load_program_file(path: Path, seen: Set[Path], loading: Set[Path]) -> Program:
    if path in seen:
        return Program(stmts=[])

    if path in loading:
        raise AxiomCompileError(f"circular import of {path}")

    if not path.exists():
        raise AxiomCompileError(f"cannot resolve import file {path}")

    loading.add(path)
    src = path.read_text(encoding="utf-8")
    program = parse_program(src, path=path)

    stmts = []
    for stmt in program.stmts:
        if isinstance(stmt, ImportStmt):
            import_path = _resolve_import_path(stmt.path, path)
            if not import_path.exists():
                raise AxiomCompileError(
                    f"cannot resolve import file {import_path}", stmt.span
                )
            imported = _load_program_file(import_path, seen, loading)
            stmts.extend(
                _namespace_module_program(imported, stmt.alias).stmts
            )
        else:
            stmts.append(stmt)

    loading.remove(path)
    seen.add(path)
    return Program(stmts=stmts)


def _resolve_import_path(raw: str, base_path: Path) -> Path:
    candidate = Path(raw)
    if candidate.is_absolute():
        raise AxiomCompileError(f"absolute import paths are not allowed: {raw!r}")
    if any(part == ".." for part in candidate.parts):
        raise AxiomCompileError(f"parent traversal in import path is not allowed: {raw!r}")
    if candidate.suffix == "":
        candidate = candidate.with_suffix(".ax")
    if candidate.suffix not in (".ax", ".AX"):
        # preserve prior behavior for module names that may include dots
        candidate = Path(str(candidate) + ".ax")
    if not candidate.is_absolute():
        candidate = base_path.parent / candidate
    return candidate


def _namespace_module_program(program: Program, module_alias: str) -> Program:
    fn_names = {
        stmt.name: f"{module_alias}.{stmt.name}"
        for stmt in program.stmts
        if isinstance(stmt, FunctionDefStmt)
    }

    def _rewrite_expr(expr: Expr):
        if isinstance(expr, IntLit):
            return expr
        if isinstance(expr, VarRef):
            return expr
        if isinstance(expr, UnaryNeg):
            return UnaryNeg(expr=_rewrite_expr(expr.expr), span=expr.span)
        if isinstance(expr, Binary):
            return Binary(
                op=expr.op,
                lhs=_rewrite_expr(expr.lhs),
                rhs=_rewrite_expr(expr.rhs),
                span=expr.span,
            )
        if isinstance(expr, CallExpr):
            callee = fn_names.get(expr.callee, expr.callee)
            return CallExpr(
                callee=callee,
                args=[_rewrite_expr(arg) for arg in expr.args],
                span=expr.span,
            )
        raise AssertionError("unknown expr")

    def _rewrite_stmt(stmt: Stmt):
        if isinstance(stmt, FunctionDefStmt):
            return FunctionDefStmt(
                name=fn_names[stmt.name],
                params=stmt.params,
                body=_rewrite_stmt(stmt.body),
                span=stmt.span,
            )
        if isinstance(stmt, LetStmt):
            return LetStmt(name=stmt.name, expr=_rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, AssignStmt):
            return AssignStmt(name=stmt.name, expr=_rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, ReturnStmt):
            return ReturnStmt(expr=_rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, PrintStmt):
            return PrintStmt(expr=_rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, ExprStmt):
            return ExprStmt(expr=_rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, BlockStmt):
            return BlockStmt(stmts=[_rewrite_stmt(s) for s in stmt.stmts], span=stmt.span)
        if isinstance(stmt, IfStmt):
            return IfStmt(
                cond=_rewrite_expr(stmt.cond),
                then_block=_rewrite_stmt(stmt.then_block),
                else_block=_rewrite_stmt(stmt.else_block) if stmt.else_block else None,
                span=stmt.span,
            )
        if isinstance(stmt, WhileStmt):
            return WhileStmt(cond=_rewrite_expr(stmt.cond), body=_rewrite_stmt(stmt.body), span=stmt.span)
        return stmt

    stmts = [_rewrite_stmt(stmt) for stmt in program.stmts]
    return Program(stmts=stmts)
