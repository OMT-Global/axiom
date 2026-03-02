from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List

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
from .bytecode import Bytecode, Instr, Op
from .errors import AxiomCompileError


@dataclass
class Compiler:
    scope_stack: List[Dict[str, int]] = field(default_factory=lambda: [{}])
    next_slot: int = 0
    strings: List[str] = field(default_factory=list)

    def compile(self, program: Program) -> Bytecode:
        self.scope_stack = [{}]
        self.next_slot = 0
        self.strings = []
        ins: List[Instr] = []
        for s in program.stmts:
            self._compile_stmt(s, ins)
        ins.append(Instr(Op.HALT))
        return Bytecode(strings=list(self.strings), instructions=ins, locals_count=self.next_slot)

    def _intern(self, s: str) -> int:
        try:
            return self.strings.index(s)
        except ValueError:
            self.strings.append(s)
            return len(self.strings) - 1

    def _resolve_slot(self, name: str, span) -> int:
        for scope in reversed(self.scope_stack):
            if name in scope:
                return scope[name]
        raise AxiomCompileError(f"undefined variable {name!r}", span)

    def _slot_for_let(self, name: str) -> int:
        current = self.scope_stack[-1]
        if name in current:
            return current[name]
        slot = self.next_slot
        self.next_slot += 1
        current[name] = slot
        self._intern(name)
        return slot

    def _compile_stmt(self, stmt, out: List[Instr]) -> None:
        if isinstance(stmt, LetStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.STORE, self._slot_for_let(stmt.name)))
            return
        if isinstance(stmt, AssignStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.STORE, self._resolve_slot(stmt.name, stmt.span)))
            return
        if isinstance(stmt, PrintStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.PRINT))
            return
        if isinstance(stmt, ExprStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.POP))
            return
        if isinstance(stmt, BlockStmt):
            self.scope_stack.append({})
            try:
                for s in stmt.stmts:
                    self._compile_stmt(s, out)
            finally:
                self.scope_stack.pop()
            return
        if isinstance(stmt, IfStmt):
            self._compile_expr(stmt.cond, out)
            jmp_false_idx = len(out)
            out.append(Instr(Op.JMP_IF_FALSE, 0))
            self._compile_stmt(stmt.then_block, out)
            if stmt.else_block is None:
                out[jmp_false_idx] = Instr(Op.JMP_IF_FALSE, len(out))
            else:
                jmp_end_idx = len(out)
                out.append(Instr(Op.JMP, 0))
                out[jmp_false_idx] = Instr(Op.JMP_IF_FALSE, len(out))
                self._compile_stmt(stmt.else_block, out)
                out[jmp_end_idx] = Instr(Op.JMP, len(out))
            return
        if isinstance(stmt, WhileStmt):
            loop_start = len(out)
            self._compile_expr(stmt.cond, out)
            jmp_false_idx = len(out)
            out.append(Instr(Op.JMP_IF_FALSE, 0))
            self._compile_stmt(stmt.body, out)
            out.append(Instr(Op.JMP, loop_start))
            out[jmp_false_idx] = Instr(Op.JMP_IF_FALSE, len(out))
            return
        raise AssertionError("unknown stmt")

    def _compile_expr(self, expr: Expr, out: List[Instr]) -> None:
        if isinstance(expr, IntLit):
            out.append(Instr(Op.CONST_I64, expr.value))
            return
        if isinstance(expr, VarRef):
            out.append(Instr(Op.LOAD, self._resolve_slot(expr.name, expr.span)))
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
            elif expr.op == BinOp.EQ:
                out.append(Instr(Op.CMP_EQ))
            elif expr.op == BinOp.NE:
                out.append(Instr(Op.CMP_NE))
            elif expr.op == BinOp.LT:
                out.append(Instr(Op.CMP_LT))
            elif expr.op == BinOp.LE:
                out.append(Instr(Op.CMP_LE))
            elif expr.op == BinOp.GT:
                out.append(Instr(Op.CMP_GT))
            elif expr.op == BinOp.GE:
                out.append(Instr(Op.CMP_GE))
            else:
                raise AssertionError("unknown binop")
            return
        raise AssertionError("unknown expr")
