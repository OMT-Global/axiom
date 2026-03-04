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
    VarRef,
    CallExpr,
    UnaryNeg,
    Binary,
    BinOp,
)
from .errors import AxiomCompileError, AxiomRuntimeError
from .intops import trunc_div, to_bool_int
from .host import HOST_BUILTINS, call_host_builtin


@dataclass
class _FunctionReturn(Exception):
    value: int


@dataclass
class Interpreter:
    RESERVED_IDENTIFIER_NAMES: ClassVar[set[str]] = {"host"}

    scopes: List[Dict[str, int]] = field(default_factory=lambda: [{}])
    global_scope: Dict[str, int] = field(default_factory=dict)
    functions: Dict[str, FunctionDefStmt] = field(default_factory=dict)
    function_depth: int = 0
    call_stack: List[Tuple[List[Dict[str, int]], int]] = field(default_factory=list)
    allow_host_side_effects: bool = False

    def run(self, program: Program, out: TextIO) -> None:
        self.global_scope = {}
        self.scopes = [self.global_scope]
        self.call_stack = []
        self.function_depth = 0
        self.functions = {}

        for stmt in program.stmts:
            if isinstance(stmt, FunctionDefStmt):
                for name in stmt.params:
                    if name in self.RESERVED_IDENTIFIER_NAMES:
                        raise AxiomRuntimeError(f"reserved identifier {name!r}")
                if stmt.name in self.RESERVED_IDENTIFIER_NAMES:
                    raise AxiomRuntimeError(f"reserved function name {stmt.name!r}")
                if stmt.name in self.functions:
                    raise AxiomCompileError(f"duplicate function {stmt.name!r}", stmt.span)
                self.functions[stmt.name] = stmt

        for stmt in program.stmts:
            if isinstance(stmt, FunctionDefStmt):
                continue
            self._exec_stmt(stmt, out)

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
            out.write(f"{self._eval(stmt.expr, out)}\n")
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
            # function declarations are pre-collected in Interpreter.run()
            return
        if isinstance(stmt, IfStmt):
            cond = self._eval(stmt.cond, out)
            if cond != 0:
                self._exec_stmt(stmt.then_block, out)
            elif stmt.else_block is not None:
                self._exec_stmt(stmt.else_block, out)
            return
        if isinstance(stmt, WhileStmt):
            while self._eval(stmt.cond, out) != 0:
                self._exec_stmt(stmt.body, out)
            return
        raise AssertionError("unknown stmt")

    def _lookup(self, name: str, span) -> int:
        for scope in reversed(self.scopes):
            if name in scope:
                return scope[name]
        raise AxiomRuntimeError(f"undefined variable {name!r}", span)

    def _assign(self, name: str, value: int, span) -> None:
        if name in self.RESERVED_IDENTIFIER_NAMES:
            raise AxiomRuntimeError(f"reserved identifier {name!r}")
        for scope in reversed(self.scopes):
            if name in scope:
                scope[name] = value
                return
        raise AxiomRuntimeError(f"undefined variable {name!r}", span)

    def _call(self, fn_name: str, args: List[int], out: TextIO) -> int:
        if fn_name.startswith("host."):
            return self._call_host(fn_name, args, out)
        if "." in fn_name:
            raise AxiomRuntimeError("only host namespace calls are supported for dotted call syntax")
        if fn_name not in self.functions:
            raise AxiomRuntimeError(f"undefined function {fn_name!r}")
        fn = self.functions[fn_name]
        if len(args) != len(fn.params):
            raise AxiomRuntimeError(
                f"function {fn_name!r} expects {len(fn.params)} args, got {len(args)}"
            )

        self.call_stack.append((self.scopes, self.function_depth))

        param_scope: Dict[str, int] = {}
        for index, name in enumerate(fn.params):
            if name in self.RESERVED_IDENTIFIER_NAMES:
                raise AxiomRuntimeError(f"reserved identifier {name!r}")
            param_scope[name] = args[index]
        self.scopes = [param_scope, self.global_scope]
        self.function_depth += 1

        try:
            self._exec_stmt(fn.body, out)
            return 0
        except _FunctionReturn as exc:
            return int(exc.value)
        finally:
            self.scopes, self.function_depth = self.call_stack.pop()

    def _call_host(self, fn_name: str, args: List[int], out: TextIO) -> int:
        host_name = fn_name[len("host.") :]
        if host_name not in HOST_BUILTINS:
            raise AxiomRuntimeError(f"undefined host function {fn_name!r}")
        arity, side_effect = HOST_BUILTINS[host_name]
        if arity != len(args):
            raise AxiomRuntimeError(
                f"host function {fn_name!r} expects {arity} args, got {len(args)}"
            )
        if side_effect and not self.allow_host_side_effects:
            raise AxiomRuntimeError(
                f"host call {fn_name!r} is side-effecting; enable allow_host_side_effects"
            )
        try:
            return call_host_builtin(host_name, args, out)
        except ValueError as e:
            raise AxiomRuntimeError(str(e)) from e

    def _eval(self, expr: Expr, out: TextIO) -> int:
        if isinstance(expr, IntLit):
            return expr.value
        if isinstance(expr, VarRef):
            return self._lookup(expr.name, expr.span)
        if isinstance(expr, CallExpr):
            args = [self._eval(arg, out) for arg in expr.args]
            return self._call(expr.callee, args, out)
        if isinstance(expr, UnaryNeg):
            return -self._eval(expr.expr, out)
        if isinstance(expr, Binary):
            a = self._eval(expr.lhs, out)
            b = self._eval(expr.rhs, out)
            if expr.op == BinOp.ADD:
                return a + b
            if expr.op == BinOp.SUB:
                return a - b
            if expr.op == BinOp.MUL:
                return a * b
            if expr.op == BinOp.DIV:
                if b == 0:
                    raise AxiomRuntimeError("division by zero", expr.span)
                return trunc_div(a, b)
            if expr.op == BinOp.EQ:
                return to_bool_int(a == b)
            if expr.op == BinOp.NE:
                return to_bool_int(a != b)
            if expr.op == BinOp.LT:
                return to_bool_int(a < b)
            if expr.op == BinOp.LE:
                return to_bool_int(a <= b)
            if expr.op == BinOp.GT:
                return to_bool_int(a > b)
            if expr.op == BinOp.GE:
                return to_bool_int(a >= b)
        raise AssertionError("unknown expr")
