from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List

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
from .bytecode import Bytecode, Instr, Op
from .errors import AxiomCompileError


@dataclass
class Compiler:
    slots: Dict[str, int] = field(default_factory=dict)  # var -> slot
    strings: List[str] = field(default_factory=list)     # debug string table

    def compile(self, program: Program) -> Bytecode:
        ins: List[Instr] = []
        for s in program.stmts:
            self._compile_stmt(s, ins)
        ins.append(Instr(Op.HALT))
        return Bytecode(strings=list(self.strings), instructions=ins, locals_count=len(self.slots))

    def _intern(self, s: str) -> int:
        try:
            return self.strings.index(s)
        except ValueError:
            self.strings.append(s)
            return len(self.strings) - 1

    def _slot_for_write(self, name: str) -> int:
        if name in self.slots:
            return self.slots[name]
        slot = len(self.slots)
        self.slots[name] = slot
        self._intern(name)
        return slot

    def _slot_for_read(self, name: str, span) -> int:
        if name not in self.slots:
            raise AxiomCompileError(f"undefined variable {name!r}", span)
        return self.slots[name]

    def _compile_stmt(self, stmt, out: List[Instr]) -> None:
        if isinstance(stmt, LetStmt):
            self._compile_expr(stmt.expr, out)
            slot = self._slot_for_write(stmt.name)
            out.append(Instr(Op.STORE, slot))
            return
        if isinstance(stmt, PrintStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.PRINT))
            return
        if isinstance(stmt, ExprStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.POP))
            return
        raise AssertionError("unknown stmt")

    def _compile_expr(self, expr: Expr, out: List[Instr]) -> None:
        if isinstance(expr, IntLit):
            out.append(Instr(Op.CONST_I64, expr.value))
            return
        if isinstance(expr, VarRef):
            slot = self._slot_for_read(expr.name, expr.span)
            out.append(Instr(Op.LOAD, slot))
            return
        if isinstance(expr, UnaryNeg):
            out.append(Instr(Op.CONST_I64, 0))
            self._compile_expr(expr.expr, out)
            out.append(Instr(Op.SUB))
            return
        if isinstance(expr, Binary):
            self._compile_expr(expr.lhs, out)
            self._compile_expr(expr.rhs, out)
            if expr.op == BinOp.ADD:
                out.append(Instr(Op.ADD))
            elif expr.op == BinOp.SUB:
                out.append(Instr(Op.SUB))
            elif expr.op == BinOp.MUL:
                out.append(Instr(Op.MUL))
            elif expr.op == BinOp.DIV:
                out.append(Instr(Op.DIV))
            else:
                raise AssertionError("unknown binop")
            return
        raise AssertionError("unknown expr")
