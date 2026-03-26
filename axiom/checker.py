from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional

from .ast import (
    ArrayLit,
    AssignStmt,
    Binary,
    BinOp,
    BlockStmt,
    BoolLit,
    CallExpr,
    Expr,
    ExprStmt,
    FunctionDefStmt,
    IfStmt,
    ImportStmt,
    IndexExpr,
    IntLit,
    LetStmt,
    Param,
    PrintStmt,
    Program,
    ReturnStmt,
    StringLit,
    TypeName,
    UnaryNeg,
    VarRef,
    WhileStmt,
    element_type,
)
from .errors import AxiomCompileError
from .host import HOST_BUILTINS
from .semantic_plan import (
    RESERVED_IDENTIFIER_NAMES,
    SemanticPlan,
    build_semantic_plan,
)
from .values import ValueKind


@dataclass(frozen=True)
class FunctionSignature:
    name: str
    param_types: List[TypeName]
    return_type: TypeName


@dataclass(frozen=True)
class CheckedProgram:
    program: Program
    expr_types: Dict[int, TypeName]
    function_signatures: Dict[str, FunctionSignature]


@dataclass
class Checker:
    allow_host_side_effects: bool = False
    allowed_host_calls: Optional[set[str]] = None
    semantic_plan: Optional[SemanticPlan] = None
    function_defs: Dict[str, FunctionDefStmt] = field(default_factory=dict)
    function_decl_order: List[str] = field(default_factory=list)
    function_scopes: Dict[str, List[Dict[str, str]]] = field(default_factory=dict)
    function_signatures: Dict[str, FunctionSignature] = field(default_factory=dict)
    checked_functions: set[str] = field(default_factory=set)
    scope_stack: List[Dict[str, TypeName]] = field(default_factory=lambda: [{}])
    function_scope_stack: List[Dict[str, str]] = field(default_factory=lambda: [{}])
    expr_types: Dict[int, TypeName] = field(default_factory=dict)
    _parent_types: Dict[str, TypeName] = field(default_factory=dict)
    _current_return_type: Optional[TypeName] = None

    def check(self, program: Program) -> CheckedProgram:
        self.semantic_plan = build_semantic_plan(
            program,
            error_factory=lambda message, span: AxiomCompileError(message, span),
        )
        self.function_defs = {}
        self.function_decl_order = []
        self.function_scopes = {}
        self.function_signatures = {}
        self.checked_functions = set()
        self.scope_stack = [{}]
        self.function_scope_stack = [{}]
        self.expr_types = {}
        self._parent_types = {}
        self._current_return_type = None

        self.function_defs = dict(self.semantic_plan.function_defs)
        self.function_decl_order = list(self.semantic_plan.function_decl_order)
        self.function_scopes = dict(self.semantic_plan.function_scopes)
        self.function_scope_stack = [dict(self.semantic_plan.global_scope)]
        for fn_name in self.function_decl_order:
            fn = self.function_defs[fn_name]
            self.function_signatures[fn_name] = FunctionSignature(
                name=fn_name,
                param_types=[
                    self._require_param_type(fn_name=fn_name, param=param)
                    for param in fn.params
                ],
                return_type=self._require_return_type(fn_name=fn_name, fn=fn),
            )

        for stmt in program.stmts:
            self._check_stmt(stmt)

        return CheckedProgram(
            program=program,
            expr_types=dict(self.expr_types),
            function_signatures=dict(self.function_signatures),
        )

    def _check_function(
        self,
        fn_name: str,
        captured_types: Dict[str, TypeName],
    ) -> None:
        if fn_name in self.checked_functions:
            return
        self.checked_functions.add(fn_name)

        fn = self.function_defs[fn_name]
        saved_scope_stack = self.scope_stack
        saved_function_scope_stack = self.function_scope_stack
        saved_parent_types = self._parent_types
        saved_return_type = self._current_return_type

        self.scope_stack = [{}]
        self.function_scope_stack = self.function_scopes.get(fn_name, [self.scope_stack[-1]])
        self._parent_types = dict(captured_types)
        signature = self.function_signatures[fn_name]
        self._current_return_type = signature.return_type

        for param in fn.params:
            self._bind_param(param)

        try:
            for stmt in fn.body.stmts:
                self._check_stmt(stmt)
            if not self._block_guarantees_return(fn.body):
                raise AxiomCompileError(
                    f"function {fn_name!r} may exit without returning {signature.return_type}",
                    fn.span,
                )
        finally:
            self.scope_stack = saved_scope_stack
            self.function_scope_stack = saved_function_scope_stack
            self._parent_types = saved_parent_types
            self._current_return_type = saved_return_type

    def _visible_types(self) -> Dict[str, TypeName]:
        types = dict(self._parent_types)
        for scope in self.scope_stack:
            types.update(scope)
        return types

    def _require_param_type(self, *, fn_name: str, param: Param) -> TypeName:
        if param.type_ref is None:
            raise AxiomCompileError(
                f"parameter {param.name!r} in function {fn_name!r} is missing a type annotation",
                param.span,
            )
        return param.type_ref.name

    def _require_return_type(self, *, fn_name: str, fn: FunctionDefStmt) -> TypeName:
        if fn.return_type is None:
            raise AxiomCompileError(
                f"function {fn_name!r} is missing a return type annotation",
                fn.span,
            )
        return fn.return_type.name

    def _require_let_type(self, stmt: LetStmt) -> TypeName:
        if stmt.type_ref is None:
            raise AxiomCompileError(
                f"let binding {stmt.name!r} is missing a type annotation",
                stmt.span,
            )
        return stmt.type_ref.name

    def _block_guarantees_return(self, block: BlockStmt) -> bool:
        for stmt in block.stmts:
            if self._stmt_guarantees_return(stmt):
                return True
        return False

    def _stmt_guarantees_return(self, stmt: object) -> bool:
        if isinstance(stmt, ReturnStmt):
            return True
        if isinstance(stmt, BlockStmt):
            return self._block_guarantees_return(stmt)
        if isinstance(stmt, IfStmt):
            return (
                stmt.else_block is not None
                and self._stmt_guarantees_return(stmt.then_block)
                and self._stmt_guarantees_return(stmt.else_block)
            )
        return False

    def _bind_param(self, param: Param) -> None:
        if param.name in RESERVED_IDENTIFIER_NAMES:
            raise AxiomCompileError(f"reserved identifier {param.name!r}", param.span)
        current = self.scope_stack[-1]
        if param.name in current:
            raise AxiomCompileError(f"duplicate parameter {param.name!r}", param.span)
        if param.type_ref is None:
            raise AxiomCompileError(
                f"parameter {param.name!r} is missing a type annotation",
                param.span,
            )
        current[param.name] = param.type_ref.name

    def _resolve_var_type(self, name: str, span) -> TypeName:
        for scope in reversed(self.scope_stack):
            if name in scope:
                return scope[name]
        if name in self._parent_types:
            return self._parent_types[name]
        raise AxiomCompileError(f"undefined variable {name!r}", span)

    def _resolve_function(self, fn_name: str, span) -> str:
        if fn_name.startswith("host."):
            return fn_name
        if self.semantic_plan is not None:
            resolved = self.semantic_plan.resolve_function(fn_name, self.function_scope_stack)
            if resolved is not None:
                return resolved
        raise AxiomCompileError(f"undefined function {fn_name!r}", span)

    def _check_stmt(self, stmt: object) -> None:
        if isinstance(stmt, LetStmt):
            if stmt.name in RESERVED_IDENTIFIER_NAMES:
                raise AxiomCompileError(f"reserved identifier {stmt.name!r}", stmt.span)
            expr_type = self._check_expr(stmt.expr)
            expected_type = self._require_let_type(stmt)
            if expr_type != expected_type:
                raise AxiomCompileError(
                    f"let binding {stmt.name!r} expects {expected_type}, got {expr_type}",
                    stmt.span,
                )
            self.scope_stack[-1][stmt.name] = expected_type
            return
        if isinstance(stmt, AssignStmt):
            expected = self._resolve_var_type(stmt.name, stmt.span)
            actual = self._check_expr(stmt.expr)
            if actual != expected:
                raise AxiomCompileError(
                    f"assignment to {stmt.name!r} expects {expected}, got {actual}",
                    stmt.span,
                )
            return
        if isinstance(stmt, ReturnStmt):
            if self._current_return_type is None:
                raise AxiomCompileError("return outside function", stmt.span)
            actual = self._check_expr(stmt.expr)
            if actual != self._current_return_type:
                raise AxiomCompileError(
                    f"return expects {self._current_return_type}, got {actual}",
                    stmt.span,
                )
            return
        if isinstance(stmt, PrintStmt):
            self._check_expr(stmt.expr)
            return
        if isinstance(stmt, ExprStmt):
            self._check_expr(stmt.expr)
            return
        if isinstance(stmt, ImportStmt):
            raise AxiomCompileError(
                "import statements are only supported in file-based compilation",
                stmt.span,
            )
        if isinstance(stmt, BlockStmt):
            self.scope_stack.append({})
            try:
                for inner in stmt.stmts:
                    self._check_stmt(inner)
            finally:
                self.scope_stack.pop()
            return
        if isinstance(stmt, FunctionDefStmt):
            fn_name = self._resolve_function(stmt.name, stmt.span)
            self._check_function(fn_name, self._visible_types())
            return
        if isinstance(stmt, IfStmt):
            cond_type = self._check_expr(stmt.cond)
            if cond_type != "bool":
                raise AxiomCompileError(
                    f"if condition expects bool, got {cond_type}",
                    stmt.cond.span,
                )
            self._check_stmt(stmt.then_block)
            if stmt.else_block is not None:
                self._check_stmt(stmt.else_block)
            return
        if isinstance(stmt, WhileStmt):
            cond_type = self._check_expr(stmt.cond)
            if cond_type != "bool":
                raise AxiomCompileError(
                    f"while condition expects bool, got {cond_type}",
                    stmt.cond.span,
                )
            self._check_stmt(stmt.body)
            return
        raise AssertionError("unknown stmt")

    def _check_host_call(self, expr: CallExpr, fn_name: str) -> TypeName:
        host_fn = fn_name[len("host.") :]
        builtin = HOST_BUILTINS.get(host_fn)
        if builtin is None:
            raise AxiomCompileError(f"undefined host function {fn_name!r}", expr.span)
        if self.allowed_host_calls is not None and host_fn not in self.allowed_host_calls:
            raise AxiomCompileError(
                f"host call {fn_name!r} is not permitted by package policy",
                expr.span,
            )
        if builtin.side_effecting and not self.allow_host_side_effects:
            raise AxiomCompileError(
                f"host call {fn_name!r} is side-effecting; pass allow_host_side_effects=True to use it",
                expr.span,
            )
        if builtin.arity != len(expr.args):
            raise AxiomCompileError(
                f"host function {fn_name!r} expects {builtin.arity} args, got {len(expr.args)}",
                expr.span,
            )
        for index, (arg, expected_kind) in enumerate(zip(expr.args, builtin.arg_kinds, strict=True)):
            actual = self._check_expr(arg)
            if not self._source_type_matches(actual, expected_kind):
                raise AxiomCompileError(
                    f"host function {fn_name!r} argument {index + 1} expects {expected_kind}, got {actual}",
                    arg.span,
                )
        if builtin.return_kind == "value":
            raise AxiomCompileError(
                f"host function {fn_name!r} returns untyped value; annotate the builtin with a concrete return kind",
                expr.span,
            )
        return builtin.return_kind

    def _source_type_matches(self, actual: TypeName, expected: ValueKind) -> bool:
        return expected == "value" or actual == expected

    def _check_expr(self, expr: Expr) -> TypeName:
        cached = self.expr_types.get(id(expr))
        if cached is not None:
            return cached

        expr_type: TypeName
        if isinstance(expr, IntLit):
            expr_type = "int"
        elif isinstance(expr, StringLit):
            expr_type = "string"
        elif isinstance(expr, BoolLit):
            expr_type = "bool"
        elif isinstance(expr, VarRef):
            expr_type = self._resolve_var_type(expr.name, expr.span)
        elif isinstance(expr, CallExpr):
            fn_name = self._resolve_function(expr.callee, expr.span)
            if fn_name.startswith("host."):
                expr_type = self._check_host_call(expr, fn_name)
            else:
                sig = self.function_signatures[fn_name]
                if len(sig.param_types) != len(expr.args):
                    raise AxiomCompileError(
                        f"function {fn_name!r} expects {len(sig.param_types)} args, got {len(expr.args)}",
                        expr.span,
                    )
                for index, (arg, expected_type) in enumerate(
                    zip(expr.args, sig.param_types, strict=True)
                ):
                    actual = self._check_expr(arg)
                    if actual != expected_type:
                        raise AxiomCompileError(
                            f"function {fn_name!r} argument {index + 1} expects {expected_type}, got {actual}",
                            arg.span,
                        )
                expr_type = sig.return_type
        elif isinstance(expr, UnaryNeg):
            inner_type = self._check_expr(expr.expr)
            if inner_type != "int":
                raise AxiomCompileError("unary '-' expects int operand", expr.span)
            expr_type = "int"
        elif isinstance(expr, Binary):
            lhs_type = self._check_expr(expr.lhs)
            rhs_type = self._check_expr(expr.rhs)
            if expr.op == BinOp.ADD:
                if lhs_type == rhs_type and lhs_type in {"int", "string"}:
                    expr_type = lhs_type
                else:
                    raise AxiomCompileError(
                        f"operator '+' expects matching int or string operands, got {lhs_type} and {rhs_type}",
                        expr.span,
                    )
            elif expr.op in (BinOp.SUB, BinOp.MUL, BinOp.DIV):
                if lhs_type != "int" or rhs_type != "int":
                    symbol = {
                        BinOp.SUB: "-",
                        BinOp.MUL: "*",
                        BinOp.DIV: "/",
                    }[expr.op]
                    raise AxiomCompileError(
                        f"operator '{symbol}' expects int operands",
                        expr.span,
                    )
                expr_type = "int"
            elif expr.op in (BinOp.LT, BinOp.LE, BinOp.GT, BinOp.GE):
                if lhs_type != "int" or rhs_type != "int":
                    symbol = {
                        BinOp.LT: "<",
                        BinOp.LE: "<=",
                        BinOp.GT: ">",
                        BinOp.GE: ">=",
                    }[expr.op]
                    raise AxiomCompileError(
                        f"operator '{symbol}' expects int operands",
                        expr.span,
                    )
                expr_type = "bool"
            elif expr.op in (BinOp.EQ, BinOp.NE):
                if lhs_type != rhs_type:
                    symbol = "==" if expr.op == BinOp.EQ else "!="
                    raise AxiomCompileError(
                        f"operator '{symbol}' expects matching operand types, got {lhs_type} and {rhs_type}",
                        expr.span,
                    )
                expr_type = "bool"
            else:
                raise AssertionError("unknown binop")
        elif isinstance(expr, ArrayLit):
            if not expr.elements:
                raise AxiomCompileError(
                    "cannot infer type of empty array literal; use a typed let binding",
                    expr.span,
                )
            first_type = self._check_expr(expr.elements[0])
            if first_type not in ("int", "string", "bool"):
                raise AxiomCompileError(
                    f"arrays of {first_type!r} are not supported",
                    expr.span,
                )
            for i, elem in enumerate(expr.elements[1:], start=1):
                elem_type = self._check_expr(elem)
                if elem_type != first_type:
                    raise AxiomCompileError(
                        f"array element {i} has type {elem_type!r}, expected {first_type!r}",
                        elem.span,
                    )
            expr_type = f"{first_type}[]"  # type: ignore[assignment]
        elif isinstance(expr, IndexExpr):
            array_type = self._check_expr(expr.array)
            if not array_type.endswith("[]"):
                raise AxiomCompileError(
                    f"cannot index non-array type {array_type!r}",
                    expr.span,
                )
            index_type = self._check_expr(expr.index)
            if index_type != "int":
                raise AxiomCompileError(
                    f"array index must be int, got {index_type!r}",
                    expr.index.span,
                )
            expr_type = element_type(array_type)
        else:
            raise AssertionError("unknown expr")

        self.expr_types[id(expr)] = expr_type
        return expr_type


def check_program(
    program: Program,
    *,
    allow_host_side_effects: bool = False,
    allowed_host_calls: Optional[set[str]] = None,
) -> CheckedProgram:
    return Checker(
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    ).check(program)
