from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Dict, List, Optional, Sequence

from .ast import FunctionDefStmt, Program, Stmt
from .errors import Span


RESERVED_FUNCTION_NAMES = frozenset({"host"})
RESERVED_IDENTIFIER_NAMES = frozenset({"host"})

ErrorFactory = Callable[[str, Optional[Span]], Exception]


@dataclass(frozen=True)
class SemanticPlan:
    global_scope: Dict[str, str]
    function_defs: Dict[str, FunctionDefStmt]
    function_decl_order: List[str]
    function_scopes: Dict[str, List[Dict[str, str]]]

    def scope_stack_for(self, fn_name: str) -> List[Dict[str, str]]:
        return self.function_scopes.get(fn_name, [self.global_scope])

    def resolve_function(self, fn_name: str, scope_stack: Sequence[Dict[str, str]]) -> Optional[str]:
        if "." not in fn_name:
            for scope in reversed(scope_stack):
                resolved = scope.get(fn_name)
                if resolved is not None:
                    return resolved
        if fn_name in self.function_defs:
            return fn_name
        return None


def build_semantic_plan(program: Program, *, error_factory: ErrorFactory) -> SemanticPlan:
    function_defs: Dict[str, FunctionDefStmt] = {}
    function_decl_order: List[str] = []
    function_scopes: Dict[str, List[Dict[str, str]]] = {}

    def qualify(parts: List[str]) -> str:
        return ".".join(parts)

    def plan_scope(stmts: List[Stmt], scope_chain: List[Dict[str, str]], scope_path: List[str]) -> Dict[str, str]:
        local_scope: Dict[str, str] = {}

        for stmt in stmts:
            if not isinstance(stmt, FunctionDefStmt):
                continue
            if stmt.name in RESERVED_FUNCTION_NAMES:
                raise error_factory(f"reserved function name {stmt.name!r}", stmt.span)
            if stmt.name in local_scope:
                raise error_factory(f"duplicate function {stmt.name!r}", stmt.span)
            for param in stmt.params:
                if param.name in RESERVED_IDENTIFIER_NAMES:
                    raise error_factory(f"reserved identifier {param.name!r}", param.span)

            qual_name = qualify(scope_path + [stmt.name])
            if qual_name in function_defs:
                raise error_factory(f"duplicate function {qual_name!r}", stmt.span)

            function_defs[qual_name] = stmt
            function_decl_order.append(qual_name)
            local_scope[stmt.name] = qual_name

        current_chain = scope_chain + [local_scope]
        for stmt in stmts:
            if not isinstance(stmt, FunctionDefStmt):
                continue
            qual_name = qualify(scope_path + [stmt.name])
            body_scope_chain = current_chain + [{stmt.name: qual_name}]
            body_locals = plan_scope(stmt.body.stmts, body_scope_chain, scope_path + [stmt.name])
            function_scopes[qual_name] = body_scope_chain + [
                body_locals,
                {stmt.name: qual_name},
            ]

        return local_scope

    global_scope = plan_scope(program.stmts, [], [])
    return SemanticPlan(
        global_scope=global_scope,
        function_defs=function_defs,
        function_decl_order=function_decl_order,
        function_scopes=function_scopes,
    )
