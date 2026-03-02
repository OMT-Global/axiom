from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, TextIO

from .ast import (
    Program,
    LetStmt,
    PrintStmt,
    ExprStmt,
    Expr,
    IntLit,
    VarRef,
    UnaryNeg,
    Binary,
    BinOp,
)
from .errors import AxiomRuntimeError


@dataclass
class Interpreter:
    env: Dict[str, int] = field(default_factory=dict)

    def run(self, program: Program, out: TextIO) -> None:
        for s in program.stmts:
            self._exec_stmt(s, out)

    def _exec_stmt(self, stmt, out: TextIO) -> None:
        if isinstance(stmt, LetStmt):
            v = self._eval(stmt.expr)
            self.env[stmt.name] = v
            return
        if isinstance(stmt, PrintStmt):
            v = self._eval(stmt.expr)
            out.write(f"{v}\n")
            return
        if isinstance(stmt, ExprStmt):
            _ = self._eval(stmt.expr)
            return
        raise AssertionError("unknown stmt")

    def _eval(self, expr: Expr) -> int:
        if isinstance(expr, IntLit):
            return expr.value
        if isinstance(expr, VarRef):
            if expr.name not in self.env:
                raise AxiomRuntimeError(f"undefined variable {expr.name!r}", expr.span)
            return self.env[expr.name]
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
                return int(a / b)
        raise AssertionError("unknown expr")
