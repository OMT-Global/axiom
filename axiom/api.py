from __future__ import annotations

from pathlib import Path
from typing import Iterable, List, Optional, Sequence, Set

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
from .errors import AxiomCompileError, AxiomError, Span


def parse_file(
    path: Path, module_search_paths: Optional[Sequence[Path]] = None
) -> Program:
    normalized_search_paths = _normalize_module_search_paths(module_search_paths)
    return _load_program_file(
        Path(path).resolve(), set(), set(), module_search_paths=normalized_search_paths
    )


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
    module_search_paths: Optional[Sequence[Path]] = None,
) -> Bytecode:
    return Compiler(
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    ).compile(
        parse_file(path, module_search_paths=module_search_paths)
    )


def _load_program_file(
    path: Path,
    seen: Set[Path],
    loading: Set[Path],
    *,
    import_source: Optional[str] = None,
    import_span: Optional[Span] = None,
    importer_path: Optional[Path] = None,
    module_search_paths: Optional[Sequence[Path]] = None,
) -> Program:
    if path in seen:
        return Program(stmts=[])

    if path in loading:
        if import_span is not None and import_source is not None and importer_path is not None:
            raise AxiomCompileError(
                f"circular import of {path}",
                import_span,
                source=import_source,
                path=str(importer_path),
            )
        raise AxiomCompileError(f"circular import of {path}")

    if not path.exists():
        if import_span is not None and import_source is not None and importer_path is not None:
            raise AxiomCompileError(
                f"cannot resolve import file {path}",
                import_span,
                source=import_source,
                path=str(importer_path),
            )
        raise AxiomCompileError(f"cannot resolve import file {path}")

    loading.add(path)
    try:
        src = path.read_text(encoding="utf-8")
        program = parse_program(src, path=path)

        stmts = []
        for stmt in program.stmts:
            if isinstance(stmt, ImportStmt):
                try:
                    import_path = _resolve_import_path(
                        stmt.path, path, module_search_paths=module_search_paths
                    )
                except AxiomCompileError as e:
                    raise AxiomCompileError(
                        e.message,
                        stmt.span,
                        source=src,
                        path=str(path),
                    ) from e
                if not import_path.exists():
                    raise AxiomCompileError(
                        f"cannot resolve import file {import_path}",
                        stmt.span,
                        source=src,
                        path=str(path),
                    )
                imported_source = import_path.read_text(encoding="utf-8")
                imported = _load_program_file(
                    import_path,
                    seen,
                    loading,
                    import_source=src,
                    import_span=stmt.span,
                    importer_path=path,
                    module_search_paths=module_search_paths,
                )
                try:
                    _validate_module_file(imported, import_path, source=imported_source)
                except AxiomError as e:
                    _attach_import_note(
                        e,
                        import_source=src,
                        import_span=stmt.span,
                        importer_path=path,
                    )
                    raise
                stmts.extend(_namespace_module_program(imported, stmt.alias).stmts)
            else:
                stmts.append(stmt)
    except AxiomError as e:
        _attach_import_note(
            e,
            import_source=import_source,
            import_span=import_span,
            importer_path=importer_path,
        )
        raise
    finally:
        loading.discard(path)

    seen.add(path)
    return Program(stmts=stmts)


def _attach_import_note(
    error: AxiomError,
    *,
    import_source: Optional[str],
    import_span: Optional[Span],
    importer_path: Optional[Path],
) -> None:
    if import_source is None or import_span is None or importer_path is None:
        return
    error.add_note(
        "imported from here",
        span=import_span,
        source=import_source,
        path=str(importer_path),
    )


def _resolve_import_path(
    raw: str, base_path: Path, module_search_paths: Optional[Sequence[Path]] = None
) -> Path:
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

    search_paths = [base_path.parent]
    search_paths.extend(module_search_paths or [])

    for root in search_paths:
        candidate_path = root / candidate
        if candidate_path.exists():
            return candidate_path
    raise AxiomCompileError(f"cannot resolve import file {candidate}")


def _normalize_module_search_paths(
    paths: Optional[Iterable[Path]] = None,
) -> Optional[Sequence[Path]]:
    if paths is None:
        return None
    normalized: List[Path] = []
    for item in paths:
        normalized.append(item)
    return normalized


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


def _validate_module_file(
    program: Program, import_path: Path, source: Optional[str] = None
) -> None:
    for stmt in program.stmts:
        if isinstance(stmt, (FunctionDefStmt, ImportStmt)):
            continue
        raise AxiomCompileError(
            f"imported module {import_path} may only contain imports and function declarations",
            stmt.span,
            path=str(import_path),
            source=source,
        )
