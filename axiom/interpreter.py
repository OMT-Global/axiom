from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, TextIO

from .ast import (
    Program,
    LetStmt,
    AssignStmt,
    PrintStmt,
    ExprStmt,
    BlockStmt,
    IfStmt,
    WhileStmt,
    Expr,
    IntLit,
    VarRef,
    UnaryNeg,
    Binary,
    BinOp,
)
from .errors import AxiomRuntimeError
from .intops import trunc_div, to_bool_int


@dataclass
class Interpreter:
    scopes: List[Dict[str, int]] = field(default_factory=lambda: [{}])

    def run(self, program: Program, out: TextIO) -> None:
        self.scopes = [{}]
        for s in program.stmts:
            self._exec_stmt(s, out)

    def _exec_stmt(self, stmt, out: TextIO) -> None:
        if isinstance(stmt, LetStmt):
            self.scopes[-1][stmt.name] = self._eval(stmt.expr)
            return
        if isinstance(stmt, AssignStmt):
            value = self._eval(stmt.expr)
            self._assign(stmt.name, value, stmt.span)
            return
        if isinstance(stmt, PrintStmt):
            out.write(f"{self._eval(stmt.expr)}\n")
            return
        if isinstance(stmt, ExprStmt):
            _ = self._eval(stmt.expr)
            return
        if isinstance(stmt, BlockStmt):
            self.scopes.append({})
            try:
                for s in stmt.stmts:
                    self._exec_stmt(s, out)
            finally:
                self.scopes.pop()
            return
        if isinstance(stmt, IfStmt):
            cond = self._eval(stmt.cond)
            if cond != 0:
                self._exec_stmt(stmt.then_block, out)
            elif stmt.else_block is not None:
                self._exec_stmt(stmt.else_block, out)
            return
        if isinstance(stmt, WhileStmt):
            while self._eval(stmt.cond) != 0:
                self._exec_stmt(stmt.body, out)
            return
        raise AssertionError("unknown stmt")

    def _lookup(self, name: str, span) -> int:
        for scope in reversed(self.scopes):
            if name in scope:
                return scope[name]
        raise AxiomRuntimeError(f"undefined variable {name!r}", span)

    def _assign(self, name: str, value: int, span) -> None:
        for scope in reversed(self.scopes):
            if name in scope:
                scope[name] = value
                return
        raise AxiomRuntimeError(f"undefined variable {name!r}", span)

    def _eval(self, expr: Expr) -> int:
        if isinstance(expr, IntLit):
            return expr.value
        if isinstance(expr, VarRef):
            return self._lookup(expr.name, expr.span)
        if isinstance(expr, UnaryNeg):
            return -self._eval(expr.expr)
        if isinstance(expr, Binary):
            a = self._eval(expr.lhs)
            b = self._eval(expr.rhs)
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
