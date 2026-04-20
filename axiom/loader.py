from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable, List, Optional, Sequence, Set

from .ast import (
    AssignStmt,
    Binary,
    BlockStmt,
    BoolLit,
    CallExpr,
    Expr,
    ExprStmt,
    FunctionDefStmt,
    IfStmt,
    ImportStmt,
    IntLit,
    LetStmt,
    PrintStmt,
    Program,
    ReturnStmt,
    Stmt,
    StringLit,
    UnaryNeg,
    VarRef,
    WhileStmt,
)
from .errors import AxiomCompileError, AxiomError, Span
from .lexer import Lexer
from .parser import Parser
from .suggestions import import_path_candidates, suggestion_suffix


def parse_program(src: str, path: Path | str | None = None) -> Program:
    toks = Lexer(src, path=str(path) if path is not None else None).tokenize()
    return Parser(
        toks,
        source=src,
        source_path=str(path) if path is not None else None,
    ).parse_program()


def normalize_module_search_paths(
    paths: Optional[Iterable[Path]] = None,
) -> Optional[Sequence[Path]]:
    if paths is None:
        return None
    normalized: List[Path] = []
    for item in paths:
        normalized.append(item)
    return normalized


def _is_relative_to(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
    except ValueError:
        return False
    return True


@dataclass
class ModuleLoader:
    module_search_paths: Optional[Sequence[Path]] = None
    seen: Set[Path] = field(default_factory=set)
    loading: Set[Path] = field(default_factory=set)
    loading_stack: List[Path] = field(default_factory=list)

    def __post_init__(self) -> None:
        self.module_search_paths = normalize_module_search_paths(self.module_search_paths)

    def load_file(self, path: Path) -> Program:
        return self._load_program_file(Path(path).resolve())

    def _load_program_file(
        self,
        path: Path,
        *,
        import_source: Optional[str] = None,
        import_span: Optional[Span] = None,
        importer_path: Optional[Path] = None,
    ) -> Program:
        if path in self.seen:
            return Program(stmts=[])

        if path in self.loading:
            message = self._circular_import_message(path)
            if import_span is not None and import_source is not None and importer_path is not None:
                raise AxiomCompileError(
                    message,
                    import_span,
                    source=import_source,
                    path=str(importer_path),
                )
            raise AxiomCompileError(message)

        if not path.exists():
            message = self._missing_import_message(path)
            if import_span is not None and import_source is not None and importer_path is not None:
                raise AxiomCompileError(
                    message,
                    import_span,
                    source=import_source,
                    path=str(importer_path),
                )
            raise AxiomCompileError(message)

        self.loading.add(path)
        self.loading_stack.append(path)
        try:
            src = path.read_text(encoding="utf-8")
            program = parse_program(src, path=path)

            stmts = []
            for stmt in program.stmts:
                if isinstance(stmt, ImportStmt):
                    try:
                        import_path = self.resolve_import_path(stmt.path, path)
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
                    imported = self._load_program_file(
                        import_path,
                        import_source=src,
                        import_span=stmt.span,
                        importer_path=path,
                    )
                    try:
                        validate_module_file(imported, import_path, source=imported_source)
                    except AxiomError as e:
                        attach_import_note(
                            e,
                            import_source=src,
                            import_span=stmt.span,
                            importer_path=path,
                        )
                        raise
                    stmts.extend(namespace_module_program(imported, stmt.alias).stmts)
                else:
                    stmts.append(stmt)
        except AxiomError as e:
            attach_import_note(
                e,
                import_source=import_source,
                import_span=import_span,
                importer_path=importer_path,
            )
            raise
        finally:
            if self.loading_stack and self.loading_stack[-1] == path:
                self.loading_stack.pop()
            else:
                self.loading_stack = [item for item in self.loading_stack if item != path]
            self.loading.discard(path)

        self.seen.add(path)
        return Program(stmts=stmts)

    def resolve_import_path(self, raw: str, base_path: Path) -> Path:
        candidate = Path(raw)
        if candidate.is_absolute():
            raise AxiomCompileError(f"absolute import paths are not allowed: {raw!r}")
        if any(part == ".." for part in candidate.parts):
            raise AxiomCompileError(f"parent traversal in import path is not allowed: {raw!r}")
        if candidate.suffix == "":
            candidate = candidate.with_suffix(".ax")
        if candidate.suffix not in (".ax", ".AX"):
            # Preserve prior behavior for module names that may include dots.
            candidate = Path(str(candidate) + ".ax")

        search_paths = [base_path.parent]
        search_paths.extend(self.module_search_paths or [])
        searched_locations: List[str] = []

        for root in search_paths:
            candidate_path = root / candidate
            searched_locations.append(str(candidate_path))
            if candidate_path.exists():
                resolved_root = root.resolve(strict=True)
                resolved_candidate = candidate_path.resolve(strict=True)
                if not _is_relative_to(resolved_candidate, resolved_root):
                    raise AxiomCompileError(
                        f"import path resolves outside module search root: {raw!r}"
                    )
                return resolved_candidate
        raise AxiomCompileError(
            self._missing_import_message(
                candidate,
                searched_locations=searched_locations,
                search_paths=search_paths,
            )
        )

    def _circular_import_message(self, path: Path) -> str:
        cycle_start = 0
        if path in self.loading_stack:
            cycle_start = self.loading_stack.index(path)
        chain = self.loading_stack[cycle_start:] + [path]
        rendered = " -> ".join(item.name for item in chain)
        return f"circular import of {path}; cycle: {rendered}"

    def _missing_import_message(
        self,
        path: Path,
        *,
        searched_locations: Optional[Sequence[str]] = None,
        search_paths: Optional[Sequence[Path]] = None,
    ) -> str:
        message = f"cannot resolve import file {path}"
        if searched_locations:
            message += f"; searched: {', '.join(searched_locations)}"
        if search_paths:
            message += suggestion_suffix(path.stem, import_path_candidates(search_paths))
        return message


def load_program_file(
    path: Path, module_search_paths: Optional[Sequence[Path]] = None
) -> Program:
    return ModuleLoader(module_search_paths=module_search_paths).load_file(path)


def attach_import_note(
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


def namespace_module_program(program: Program, module_alias: str) -> Program:
    fn_names = {
        stmt.name: f"{module_alias}.{stmt.name}"
        for stmt in program.stmts
        if isinstance(stmt, FunctionDefStmt)
    }

    def rewrite_expr(expr: Expr) -> Expr:
        if isinstance(expr, (IntLit, StringLit, BoolLit, VarRef)):
            return expr
        if isinstance(expr, UnaryNeg):
            return UnaryNeg(expr=rewrite_expr(expr.expr), span=expr.span)
        if isinstance(expr, Binary):
            return Binary(
                op=expr.op,
                lhs=rewrite_expr(expr.lhs),
                rhs=rewrite_expr(expr.rhs),
                span=expr.span,
            )
        if isinstance(expr, CallExpr):
            callee = fn_names.get(expr.callee, expr.callee)
            return CallExpr(
                callee=callee,
                args=[rewrite_expr(arg) for arg in expr.args],
                span=expr.span,
            )
        raise AssertionError("unknown expr")

    def rewrite_stmt(stmt: Stmt) -> Stmt:
        if isinstance(stmt, FunctionDefStmt):
            return FunctionDefStmt(
                name=fn_names[stmt.name],
                params=stmt.params,
                return_type=stmt.return_type,
                body=rewrite_stmt(stmt.body),
                span=stmt.span,
            )
        if isinstance(stmt, LetStmt):
            return LetStmt(
                name=stmt.name,
                type_ref=stmt.type_ref,
                expr=rewrite_expr(stmt.expr),
                span=stmt.span,
            )
        if isinstance(stmt, AssignStmt):
            return AssignStmt(name=stmt.name, expr=rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, ReturnStmt):
            return ReturnStmt(expr=rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, PrintStmt):
            return PrintStmt(expr=rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, ExprStmt):
            return ExprStmt(expr=rewrite_expr(stmt.expr), span=stmt.span)
        if isinstance(stmt, BlockStmt):
            return BlockStmt(stmts=[rewrite_stmt(item) for item in stmt.stmts], span=stmt.span)
        if isinstance(stmt, IfStmt):
            return IfStmt(
                cond=rewrite_expr(stmt.cond),
                then_block=rewrite_stmt(stmt.then_block),
                else_block=rewrite_stmt(stmt.else_block) if stmt.else_block else None,
                span=stmt.span,
            )
        if isinstance(stmt, WhileStmt):
            return WhileStmt(
                cond=rewrite_expr(stmt.cond),
                body=rewrite_stmt(stmt.body),
                span=stmt.span,
            )
        return stmt

    return Program(stmts=[rewrite_stmt(stmt) for stmt in program.stmts])


def validate_module_file(
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
