from __future__ import annotations

from dataclasses import dataclass, field
from typing import ClassVar, Dict, List, TextIO, Tuple

from .ast import (
    Program,
    LetStmt,
    ImportStmt,
    AssignStmt,
    PrintStmt,
    ReturnStmt,
    FunctionDefStmt,
    ExprStmt,
    BlockStmt,
    IfStmt,
    WhileStmt,
    Expr,
    IntLit,
    StringLit,
    BoolLit,
    VarRef,
    CallExpr,
    UnaryNeg,
    Binary,
    BinOp,
)
from .errors import AxiomRuntimeError
from .host import HOST_BUILTINS, call_host_builtin
from .values import (
    Value,
    add_values,
    compare_eq,
    compare_ge,
    compare_gt,
    compare_le,
    compare_lt,
    compare_ne,
    div_values,
    mul_values,
    negate_value,
    render_value,
    require_condition_bool,
    sub_values,
)


@dataclass
class _FunctionReturn(Exception):
    value: Value


@dataclass
class Interpreter:
    RESERVED_IDENTIFIER_NAMES: ClassVar[set[str]] = {"host"}

    scopes: List[Dict[str, Value]] = field(default_factory=lambda: [{}])
    global_scope: Dict[str, Value] = field(default_factory=dict)
    functions: Dict[str, FunctionDefStmt] = field(default_factory=dict)
    function_scope_stack: List[Dict[str, str]] = field(default_factory=lambda: [{}])
    function_scopes: Dict[str, List[Dict[str, str]]] = field(default_factory=dict)
    function_depth: int = 0
    call_stack: List[Tuple[List[Dict[str, Value]], int, List[Dict[str, str]]]] = field(
        default_factory=list
    )
    allow_host_side_effects: bool = False

    def run(self, program: Program, out: TextIO) -> None:
        self.global_scope = {}
        self.scopes = [self.global_scope]
        self.call_stack = []
        self.function_depth = 0
        self.functions = {}
        self.function_scopes = {}
        self.function_scope_stack = [{}]

        global_scope = self._collect_functions(program.stmts, [], [])
        self.function_scope_stack = [global_scope]

        for stmt in program.stmts:
            if isinstance(stmt, FunctionDefStmt):
                continue
            self._exec_stmt(stmt, out)

    def _collect_functions(
        self, stmts: List, scope_chain: List[Dict[str, str]], scope_path: List[str]
    ) -> Dict[str, str]:
        local_scope: Dict[str, str] = {}

        for stmt in stmts:
            if not isinstance(stmt, FunctionDefStmt):
                continue
            if stmt.name in self.RESERVED_IDENTIFIER_NAMES:
                raise AxiomRuntimeError(f"reserved function name {stmt.name!r}")
            if any(param.name in self.RESERVED_IDENTIFIER_NAMES for param in stmt.params):
                for param in stmt.params:
                    if param.name in self.RESERVED_IDENTIFIER_NAMES:
                        raise AxiomRuntimeError(f"reserved identifier {param.name!r}")
            if stmt.name in local_scope:
                raise AxiomRuntimeError(f"duplicate function {stmt.name!r}")

            qual_name = ".".join(scope_path + [stmt.name])
            if qual_name in self.functions:
                raise AxiomRuntimeError(f"duplicate function {qual_name!r}")

            self.functions[qual_name] = stmt
            local_scope[stmt.name] = qual_name

        current_chain = scope_chain + [local_scope]

        for stmt in stmts:
            if not isinstance(stmt, FunctionDefStmt):
                continue
            qual_name = ".".join(scope_path + [stmt.name])
            body_scope_chain = current_chain + [{stmt.name: qual_name}]
            body_locals = self._collect_functions(
                stmt.body.stmts, body_scope_chain, scope_path + [stmt.name]
            )
            self.function_scopes[qual_name] = body_scope_chain + [
                body_locals,
                {stmt.name: qual_name},
            ]

        return local_scope

    def _resolve_function(self, name: str) -> str:
        if "." in name and name in self.functions:
            return name
        for scope in reversed(self.function_scope_stack):
            if name in scope:
                return scope[name]
        raise AxiomRuntimeError(f"undefined function {name!r}")

    def _exec_stmt(self, stmt, out: TextIO) -> None:
        if isinstance(stmt, LetStmt):
            if stmt.name in self.RESERVED_IDENTIFIER_NAMES:
                raise AxiomRuntimeError(f"reserved identifier {stmt.name!r}")
            self.scopes[-1][stmt.name] = self._eval(stmt.expr, out)
            return
        if isinstance(stmt, AssignStmt):
            value = self._eval(stmt.expr, out)
            self._assign(stmt.name, value, stmt.span)
            return
        if isinstance(stmt, ReturnStmt):
            if self.function_depth == 0:
                raise AxiomRuntimeError("return outside function", stmt.span)
            raise _FunctionReturn(self._eval(stmt.expr, out))
        if isinstance(stmt, PrintStmt):
            out.write(f"{render_value(self._eval(stmt.expr, out))}\n")
            return
        if isinstance(stmt, ExprStmt):
            self._eval(stmt.expr, out)
            return
        if isinstance(stmt, ImportStmt):
            raise AxiomRuntimeError("import statements are only supported in file-based parsing")
        if isinstance(stmt, BlockStmt):
            self.scopes.append({})
            try:
                for s in stmt.stmts:
                    self._exec_stmt(s, out)
            finally:
                self.scopes.pop()
            return
        if isinstance(stmt, FunctionDefStmt):
            return
        if isinstance(stmt, IfStmt):
            cond = self._eval(stmt.cond, out)
            try:
                cond_value = require_condition_bool(cond, context="if condition")
            except ValueError as e:
                raise AxiomRuntimeError(str(e), stmt.cond.span) from e
            if cond_value:
                self._exec_stmt(stmt.then_block, out)
            elif stmt.else_block is not None:
                self._exec_stmt(stmt.else_block, out)
            return
        if isinstance(stmt, WhileStmt):
            while True:
                try:
                    cond_value = require_condition_bool(
                        self._eval(stmt.cond, out), context="while condition"
                    )
                except ValueError as e:
                    raise AxiomRuntimeError(str(e), stmt.cond.span) from e
                if not cond_value:
                    break
                self._exec_stmt(stmt.body, out)
            return
        raise AssertionError("unknown stmt")

    def _lookup(self, name: str, span) -> Value:
        for scope in reversed(self.scopes):
            if name in scope:
                return scope[name]
        raise AxiomRuntimeError(f"undefined variable {name!r}", span)

    def _assign(self, name: str, value: Value, span) -> None:
        if name in self.RESERVED_IDENTIFIER_NAMES:
            raise AxiomRuntimeError(f"reserved identifier {name!r}")
        for scope in reversed(self.scopes):
            if name in scope:
                scope[name] = value
                return
        raise AxiomRuntimeError(f"undefined variable {name!r}", span)

    def _call(self, fn_name: str, args: List[Value], out: TextIO) -> Value:
        if fn_name.startswith("host."):
            return self._call_host(fn_name, args, out)
        fn_name = self._resolve_function(fn_name)
        if fn_name not in self.functions:
            raise AxiomRuntimeError(f"undefined function {fn_name!r}")
        fn = self.functions[fn_name]
        if len(args) != len(fn.params):
            raise AxiomRuntimeError(
                f"function {fn_name!r} expects {len(fn.params)} args, got {len(args)}"
            )

        self.call_stack.append((self.scopes, self.function_depth, self.function_scope_stack))

        param_scope: Dict[str, Value] = {}
        for index, param in enumerate(fn.params):
            if param.name in self.RESERVED_IDENTIFIER_NAMES:
                raise AxiomRuntimeError(f"reserved identifier {param.name!r}")
            param_scope[param.name] = args[index]

        self.scopes.append(param_scope)
        self.function_scope_stack = self.function_scopes.get(
            fn_name, [param_scope]
        )
        self.function_depth += 1

        try:
            self._exec_stmt(fn.body, out)
            return self._default_return_value(fn)
        except _FunctionReturn as exc:
            return exc.value
        finally:
            self.scopes, self.function_depth, self.function_scope_stack = self.call_stack.pop()

    def _default_return_value(self, fn: FunctionDefStmt) -> Value:
        if fn.return_type is None or fn.return_type.name == "int":
            return 0
        if fn.return_type.name == "string":
            return ""
        if fn.return_type.name == "bool":
            return False
        raise AxiomRuntimeError(f"unsupported return type {fn.return_type.name!r}")

    def _call_host(self, fn_name: str, args: List[Value], out: TextIO) -> Value:
        host_name = fn_name[len("host.") :]
        builtin = HOST_BUILTINS.get(host_name)
        if builtin is None:
            raise AxiomRuntimeError(f"undefined host function {fn_name!r}")
        if builtin.arity != len(args):
            raise AxiomRuntimeError(
                f"host function {fn_name!r} expects {builtin.arity} args, got {len(args)}"
            )
        if builtin.side_effecting and not self.allow_host_side_effects:
            raise AxiomRuntimeError(
                f"host call {fn_name!r} is side-effecting; enable allow_host_side_effects"
            )
        try:
            return call_host_builtin(host_name, args, out)
        except ValueError as e:
            raise AxiomRuntimeError(str(e)) from e

    def _eval(self, expr: Expr, out: TextIO) -> Value:
        if isinstance(expr, IntLit):
            return expr.value
        if isinstance(expr, StringLit):
            return expr.value
        if isinstance(expr, BoolLit):
            return expr.value
        if isinstance(expr, VarRef):
            return self._lookup(expr.name, expr.span)
        if isinstance(expr, CallExpr):
            args = [self._eval(arg, out) for arg in expr.args]
            return self._call(expr.callee, args, out)
        if isinstance(expr, UnaryNeg):
            try:
                return negate_value(self._eval(expr.expr, out), context="unary '-'")
            except ValueError as e:
                raise AxiomRuntimeError(str(e), expr.span) from e
        if isinstance(expr, Binary):
            a = self._eval(expr.lhs, out)
            b = self._eval(expr.rhs, out)
            try:
                if expr.op == BinOp.ADD:
                    return add_values(a, b, context="operator '+'")
                if expr.op == BinOp.SUB:
                    return sub_values(a, b, context="operator '-'")
                if expr.op == BinOp.MUL:
                    return mul_values(a, b, context="operator '*'")
                if expr.op == BinOp.DIV:
                    return div_values(a, b, context="operator '/'")
                if expr.op == BinOp.EQ:
                    return compare_eq(a, b)
                if expr.op == BinOp.NE:
                    return compare_ne(a, b)
                if expr.op == BinOp.LT:
                    return compare_lt(a, b, context="operator '<'")
                if expr.op == BinOp.LE:
                    return compare_le(a, b, context="operator '<='")
                if expr.op == BinOp.GT:
                    return compare_gt(a, b, context="operator '>'")
                if expr.op == BinOp.GE:
                    return compare_ge(a, b, context="operator '>='")
            except ValueError as e:
                raise AxiomRuntimeError(str(e), expr.span) from e
        raise AssertionError("unknown expr")
