from __future__ import annotations

from dataclasses import dataclass, field
from typing import ClassVar, Dict, List, Optional, Set

from .ast import (
    Program,
    LetStmt,
    ImportStmt,
    AssignStmt,
    PrintStmt,
    ExprStmt,
    ReturnStmt,
    FunctionDefStmt,
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
from .bytecode import Bytecode, FunctionMeta, Instr, Op
from .errors import AxiomCompileError
from .host import HOST_BUILTINS


@dataclass
class Compiler:
    scope_stack: List[Dict[str, int]] = field(default_factory=lambda: [{}])
    next_slot: int = 0
    strings: List[str] = field(default_factory=list)
    function_ids: Dict[str, int] = field(default_factory=dict)
    function_arities: Dict[str, int] = field(default_factory=dict)
    function_defs: List[FunctionDefStmt] = field(default_factory=list)
    functions: List[FunctionMeta] = field(default_factory=list)
    function_locals: Dict[str, int] = field(default_factory=dict)
    allow_host_side_effects: bool = False
    allowed_host_calls: Optional[Set[str]] = None
    RESERVED_FUNCTION_NAMES: ClassVar[set[str]] = {"host"}
    RESERVED_IDENTIFIER_NAMES: ClassVar[set[str]] = {"host"}

    def compile(self, program: Program) -> Bytecode:
        self.scope_stack = [{}]
        self.next_slot = 0
        self.strings = []
        self.function_ids = {}
        self.function_arities = {}
        self.function_defs = []
        self.functions = []
        self.function_locals = {}

        for stmt in program.stmts:
            if isinstance(stmt, FunctionDefStmt):
                self._register_function(stmt)

        out: List[Instr] = []
        for stmt in program.stmts:
            if isinstance(stmt, FunctionDefStmt):
                continue
            self._compile_stmt(stmt, out, in_function=False, in_toplevel=True)
        out.append(Instr(Op.HALT))

        for fn in self.function_defs:
            entry = len(out)
            self._compile_function(fn, out)
            self.functions.append(
                FunctionMeta(
                    name_index=self._intern(fn.name),
                    entry=entry,
                    arity=len(fn.params),
                    locals_count=self.function_locals[fn.name],
                )
            )

        return Bytecode(
            strings=list(self.strings),
            instructions=out,
            locals_count=self.next_slot,
            functions=self.functions,
        )

    def _register_function(self, fn: FunctionDefStmt) -> None:
        if fn.name in self.RESERVED_FUNCTION_NAMES:
            raise AxiomCompileError(f"reserved function name {fn.name!r}", fn.span)
        if fn.name in self.function_ids:
            raise AxiomCompileError(f"duplicate function {fn.name!r}", fn.span)
        self.function_ids[fn.name] = len(self.function_defs)
        self.function_arities[fn.name] = len(fn.params)
        self.function_defs.append(fn)

    def _compile_function(self, fn: FunctionDefStmt, out: List[Instr]) -> None:
        locals_count_before = self.next_slot
        scope_stack_before = self.scope_stack
        self.scope_stack = [{}]
        self.next_slot = 0
        function_start = len(out)

        for p in fn.params:
            self._slot_for_param(p, fn.span)

        for stmt in fn.body.stmts:
            self._compile_stmt(stmt, out, in_function=True, in_toplevel=False)
        if len(out) == function_start or out[-1].op != Op.RET:
            out.append(Instr(Op.CONST_I64, 0))
            out.append(Instr(Op.RET))

        function_locals = self.next_slot
        self.function_locals[fn.name] = function_locals

        self.scope_stack = scope_stack_before
        self.next_slot = locals_count_before

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

    def _slot_for_let(self, name: str, span) -> int:
        if name in self.RESERVED_IDENTIFIER_NAMES:
            raise AxiomCompileError(f"reserved identifier {name!r}", span)
        current = self.scope_stack[-1]
        if name in current:
            return current[name]
        slot = self.next_slot
        self.next_slot += 1
        current[name] = slot
        self._intern(name)
        return slot

    def _slot_for_param(self, name: str, span) -> None:
        if name in self.RESERVED_IDENTIFIER_NAMES:
            raise AxiomCompileError(f"reserved identifier {name!r}", span)
        current = self.scope_stack[-1]
        if name in current:
            raise AxiomCompileError(f"duplicate parameter {name!r}", span)
        slot = self.next_slot
        self.next_slot += 1
        current[name] = slot
        self._intern(name)

    def _compile_stmt(self, stmt, out: List[Instr], in_function: bool, in_toplevel: bool) -> None:
        if isinstance(stmt, LetStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.STORE, self._slot_for_let(stmt.name, stmt.span)))
            return
        if isinstance(stmt, AssignStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.STORE, self._resolve_slot(stmt.name, stmt.span)))
            return
        if isinstance(stmt, ReturnStmt):
            if not in_function:
                raise AxiomCompileError("return outside function", stmt.span)
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.RET))
            return
        if isinstance(stmt, PrintStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.PRINT))
            return
        if isinstance(stmt, ExprStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.POP))
            return
        if isinstance(stmt, ImportStmt):
            raise AxiomCompileError(
                "import statements are only supported in file-based compilation",
                stmt.span,
            )
        if isinstance(stmt, BlockStmt):
            self.scope_stack.append({})
            try:
                for s in stmt.stmts:
                    self._compile_stmt(s, out, in_function=in_function, in_toplevel=False)
            finally:
                self.scope_stack.pop()
            return
        if isinstance(stmt, FunctionDefStmt):
            if not in_toplevel:
                raise AxiomCompileError("nested function definitions are not supported", stmt.span)
            return
        if isinstance(stmt, IfStmt):
            self._compile_expr(stmt.cond, out)
            jmp_false_idx = len(out)
            out.append(Instr(Op.JMP_IF_FALSE, 0))
            self._compile_stmt(stmt.then_block, out, in_function=in_function, in_toplevel=False)
            if stmt.else_block is None:
                out[jmp_false_idx] = Instr(Op.JMP_IF_FALSE, len(out))
            else:
                jmp_end_idx = len(out)
                out.append(Instr(Op.JMP, 0))
                out[jmp_false_idx] = Instr(Op.JMP_IF_FALSE, len(out))
                self._compile_stmt(stmt.else_block, out, in_function=in_function, in_toplevel=False)
                out[jmp_end_idx] = Instr(Op.JMP, len(out))
            return
        if isinstance(stmt, WhileStmt):
            loop_start = len(out)
            self._compile_expr(stmt.cond, out)
            jmp_false_idx = len(out)
            out.append(Instr(Op.JMP_IF_FALSE, 0))
            self._compile_stmt(stmt.body, out, in_function=in_function, in_toplevel=False)
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
        if isinstance(expr, CallExpr):
            fn_name = expr.callee
            if fn_name.startswith("host."):
                host_fn = fn_name[len("host.") :]
                if host_fn not in HOST_BUILTINS:
                    raise AxiomCompileError(f"undefined host function {fn_name!r}", expr.span)
                if self.allowed_host_calls is not None and host_fn not in self.allowed_host_calls:
                    raise AxiomCompileError(
                        f"host call {fn_name!r} is not permitted by package policy",
                        expr.span,
                    )
                arity, side_effectful = HOST_BUILTINS[host_fn]
                if side_effectful and not self.allow_host_side_effects:
                    raise AxiomCompileError(
                        f"host call {fn_name!r} is side-effecting; pass allow_host_side_effects=True to use it",
                        expr.span,
                    )
                if arity != len(expr.args):
                    raise AxiomCompileError(
                        f"host function {fn_name!r} expects {arity} args, got {len(expr.args)}",
                        expr.span,
                    )
                for arg in expr.args:
                    self._compile_expr(arg, out)
                out.append(Instr(Op.HOST_CALL, self._intern(host_fn)))
                return
            if "." in fn_name:
                raise AxiomCompileError(f"only host namespace calls are supported: {fn_name!r}", expr.span)
            if fn_name not in self.function_ids:
                raise AxiomCompileError(f"undefined function {fn_name!r}", expr.span)
            arity = self.function_arities[fn_name]
            if arity != len(expr.args):
                raise AxiomCompileError(
                    f"function {fn_name!r} expects {arity} args, got {len(expr.args)}",
                    expr.span,
                )
            for arg in expr.args:
                self._compile_expr(arg, out)
            out.append(Instr(Op.CALL, self.function_ids[fn_name]))
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
