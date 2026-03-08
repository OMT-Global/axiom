from __future__ import annotations

from dataclasses import dataclass, field
from typing import ClassVar, Dict, List, Optional, Set

from .ast import (
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
    IntLit,
    LetStmt,
    Param,
    PrintStmt,
    ReturnStmt,
    StringLit,
    TypeName,
    UnaryNeg,
    VarRef,
    WhileStmt,
)
from .bytecode import Bytecode, FunctionMeta, Instr, ModuleMeta, Op, Upvalue
from .checker import CheckedProgram, FunctionSignature
from .errors import AxiomCompileError


@dataclass(frozen=True)
class BindingRef:
    from_local: bool
    index: int


@dataclass
class Compiler:
    scope_stack: List[Dict[str, int]] = field(default_factory=lambda: [{}])
    next_slot: int = 0
    strings: List[str] = field(default_factory=list)
    function_ids: Dict[str, int] = field(default_factory=dict)
    function_defs: Dict[str, FunctionDefStmt] = field(default_factory=dict)
    function_decl_order: List[str] = field(default_factory=list)
    function_signatures: Dict[str, FunctionSignature] = field(default_factory=dict)
    function_metas: Dict[str, FunctionMeta] = field(default_factory=dict)
    function_locals: Dict[str, int] = field(default_factory=dict)
    function_upvalues: Dict[str, List[Upvalue]] = field(default_factory=dict)
    function_upvalue_indices: Dict[str, Dict[str, int]] = field(default_factory=dict)
    function_scopes: Dict[str, List[Dict[str, str]]] = field(default_factory=dict)
    function_scope_stack: List[Dict[str, str]] = field(default_factory=lambda: [{}])
    compiled_functions: Set[str] = field(default_factory=set)
    allow_host_side_effects: bool = False
    allowed_host_calls: Optional[Set[str]] = None
    _parent_bindings: Dict[str, BindingRef] = field(default_factory=dict)
    _current_upvalue_map: Dict[str, int] = field(default_factory=dict)
    _current_upvalues: List[Upvalue] = field(default_factory=list)
    RESERVED_FUNCTION_NAMES: ClassVar[set[str]] = {"host"}
    RESERVED_IDENTIFIER_NAMES: ClassVar[set[str]] = {"host"}

    def compile(self, checked: CheckedProgram) -> Bytecode:
        program = checked.program
        self.scope_stack = [{}]
        self.next_slot = 0
        self.strings = []
        self.function_ids = {}
        self.function_defs = {}
        self.function_decl_order = []
        self.function_signatures = dict(checked.function_signatures)
        self.function_metas = {}
        self.function_locals = {}
        self.function_upvalues = {}
        self.function_upvalue_indices = {}
        self.function_scopes = {}
        self.function_scope_stack = [{}]
        self.compiled_functions = set()
        self._parent_bindings = {}
        self._current_upvalue_map = {}
        self._current_upvalues = []

        global_scope = self._collect_functions(program.stmts, [], [])
        self.function_scope_stack = [global_scope]

        out: List[Instr] = []
        for stmt in program.stmts:
            self._compile_stmt(stmt, out, in_function=False)
        out.append(Instr(Op.HALT))

        for fn_name in self.function_decl_order:
            if fn_name not in self.compiled_functions:
                raise AxiomCompileError(f"function {fn_name!r} was not compiled")

        ordered_functions: List[FunctionMeta] = []
        for fn_name in self.function_decl_order:
            meta = self.function_metas.get(fn_name)
            if meta is None:
                raise AxiomCompileError(
                    f"internal compiler failure: missing function {fn_name!r}"
                )
            ordered_functions.append(meta)

        module_map: Dict[str, List[int]] = {}
        for index, function_meta in enumerate(ordered_functions):
            function_name = self.strings[function_meta.name_index]
            if "." in function_name:
                namespace = function_name.rsplit(".", 1)[0]
                if namespace:
                    module_map.setdefault(namespace, []).append(index)

        module_metas = [
            ModuleMeta(
                namespace_index=self._intern(namespace),
                function_indices=indices,
            )
            for namespace, indices in sorted(module_map.items())
        ]

        return Bytecode(
            strings=list(self.strings),
            instructions=out,
            locals_count=self.next_slot,
            functions=ordered_functions,
            modules=module_metas,
        )

    def _qualify(self, parts: List[str]) -> str:
        return ".".join(parts)

    def _collect_functions(
        self, stmts: List[object], scope_chain: List[Dict[str, str]], scope_path: List[str]
    ) -> Dict[str, str]:
        local_scope: Dict[str, str] = {}

        for stmt in stmts:
            if not isinstance(stmt, FunctionDefStmt):
                continue
            if stmt.name in self.RESERVED_FUNCTION_NAMES:
                raise AxiomCompileError(f"reserved function name {stmt.name!r}", stmt.span)
            if stmt.name in local_scope:
                raise AxiomCompileError(f"duplicate function {stmt.name!r}", stmt.span)

            qual_name = self._qualify(scope_path + [stmt.name])
            if qual_name in self.function_ids:
                raise AxiomCompileError(f"duplicate function {qual_name!r}", stmt.span)

            self.function_ids[qual_name] = len(self.function_decl_order)
            self.function_defs[qual_name] = stmt
            self.function_decl_order.append(qual_name)
            local_scope[stmt.name] = qual_name

        current_chain = scope_chain + [local_scope]
        for stmt in stmts:
            if not isinstance(stmt, FunctionDefStmt):
                continue
            qual_name = self._qualify(scope_path + [stmt.name])
            body_scope_chain = current_chain + [{stmt.name: qual_name}]
            body_locals = self._collect_functions(
                stmt.body.stmts, body_scope_chain, scope_path + [stmt.name]
            )
            self.function_scopes[qual_name] = body_scope_chain + [
                body_locals,
                {stmt.name: qual_name},
            ]

        return local_scope

    def _compile_function(
        self,
        fn: FunctionDefStmt,
        out: List[Instr],
        fn_name: str,
        parent_bindings: Dict[str, BindingRef],
    ) -> None:
        if fn_name in self.compiled_functions:
            return
        self.compiled_functions.add(fn_name)

        saved_scope_stack = self.scope_stack
        saved_next_slot = self.next_slot
        saved_function_scope_stack = self.function_scope_stack
        saved_parent_bindings = self._parent_bindings
        saved_upvalue_map = self._current_upvalue_map
        saved_upvalues = self._current_upvalues

        self.scope_stack = [{}]
        self.next_slot = 0
        self.function_scope_stack = self.function_scopes.get(fn_name, [self.scope_stack[-1]])
        self._current_upvalues = []
        self._current_upvalue_map = {}
        self._parent_bindings = {}

        for name, source in parent_bindings.items():
            up_idx = self._ensure_upvalue(name, source)
            self._parent_bindings[name] = BindingRef(False, up_idx)

        for param in fn.params:
            self._slot_for_param(param)

        function_start = len(out)
        for stmt in fn.body.stmts:
            self._compile_stmt(stmt, out, in_function=True)
        if len(out) == function_start or out[-1].op != Op.RET:
            self._emit_default_return(fn_name, out)

        function_locals = self.next_slot
        self.function_locals[fn_name] = function_locals
        self.function_upvalues[fn_name] = list(self._current_upvalues)
        self.function_upvalue_indices[fn_name] = dict(self._current_upvalue_map)
        signature = self.function_signatures[fn_name]
        self.function_metas[fn_name] = FunctionMeta(
            name_index=self._intern(fn_name),
            entry=function_start,
            arity=len(signature.param_types),
            locals_count=function_locals,
            upvalues=list(self._current_upvalues),
        )

        self.scope_stack = saved_scope_stack
        self.next_slot = saved_next_slot
        self.function_scope_stack = saved_function_scope_stack
        self._parent_bindings = saved_parent_bindings
        self._current_upvalue_map = saved_upvalue_map
        self._current_upvalues = saved_upvalues

    def _emit_default_return(self, fn_name: str, out: List[Instr]) -> None:
        return_type = self.function_signatures[fn_name].return_type
        if return_type == "int":
            out.append(Instr(Op.CONST_I64, 0))
        elif return_type == "string":
            out.append(Instr(Op.CONST_STRING, self._intern("")))
        elif return_type == "bool":
            out.append(Instr(Op.CONST_BOOL, 0))
        else:
            raise AxiomCompileError(f"unsupported return type {return_type!r}")
        out.append(Instr(Op.RET))

    def _intern(self, s: str) -> int:
        try:
            return self.strings.index(s)
        except ValueError:
            self.strings.append(s)
            return len(self.strings) - 1

    def _ensure_upvalue(self, name: str, source: BindingRef) -> int:
        existing = self._current_upvalue_map.get(name)
        if existing is not None:
            return existing
        index = len(self._current_upvalues)
        self._current_upvalues.append(
            Upvalue(from_local=source.from_local, index=source.index)
        )
        self._current_upvalue_map[name] = index
        return index

    def _visible_bindings(self) -> Dict[str, BindingRef]:
        bindings = dict(self._parent_bindings)
        for scope in self.scope_stack:
            for name, slot in scope.items():
                bindings[name] = BindingRef(True, slot)
        return bindings

    def _resolve_var(self, name: str, span) -> tuple[int, bool]:
        for scope in reversed(self.scope_stack):
            if name in scope:
                return scope[name], True
        source = self._parent_bindings.get(name)
        if source is None:
            raise AxiomCompileError(f"undefined variable {name!r}", span)
        return source.index, False

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

    def _slot_for_param(self, param: Param) -> None:
        if param.name in self.RESERVED_IDENTIFIER_NAMES:
            raise AxiomCompileError(f"reserved identifier {param.name!r}", param.span)
        current = self.scope_stack[-1]
        if param.name in current:
            raise AxiomCompileError(f"duplicate parameter {param.name!r}", param.span)
        slot = self.next_slot
        self.next_slot += 1
        current[param.name] = slot
        self._intern(param.name)

    def _compile_stmt(self, stmt: object, out: List[Instr], *, in_function: bool) -> None:
        if isinstance(stmt, LetStmt):
            self._compile_expr(stmt.expr, out)
            out.append(Instr(Op.STORE, self._slot_for_let(stmt.name, stmt.span)))
            return
        if isinstance(stmt, AssignStmt):
            self._compile_expr(stmt.expr, out)
            slot, is_local = self._resolve_var(stmt.name, stmt.span)
            out.append(Instr(Op.STORE if is_local else Op.STORE_UPVALUE, slot))
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
                for inner in stmt.stmts:
                    self._compile_stmt(inner, out, in_function=in_function)
            finally:
                self.scope_stack.pop()
            return
        if isinstance(stmt, FunctionDefStmt):
            skip_idx = len(out)
            out.append(Instr(Op.JMP, 0))
            fn_name = self._resolve_function(stmt.name, stmt.span)
            self._compile_function(
                self.function_defs[fn_name],
                out,
                fn_name,
                self._visible_bindings(),
            )
            out[skip_idx] = Instr(Op.JMP, len(out))
            return
        if isinstance(stmt, IfStmt):
            self._compile_expr(stmt.cond, out)
            jmp_false_idx = len(out)
            out.append(Instr(Op.JMP_IF_FALSE, 0))
            self._compile_stmt(stmt.then_block, out, in_function=in_function)
            if stmt.else_block is None:
                out[jmp_false_idx] = Instr(Op.JMP_IF_FALSE, len(out))
            else:
                jmp_end_idx = len(out)
                out.append(Instr(Op.JMP, 0))
                out[jmp_false_idx] = Instr(Op.JMP_IF_FALSE, len(out))
                self._compile_stmt(stmt.else_block, out, in_function=in_function)
                out[jmp_end_idx] = Instr(Op.JMP, len(out))
            return
        if isinstance(stmt, WhileStmt):
            loop_start = len(out)
            self._compile_expr(stmt.cond, out)
            jmp_false_idx = len(out)
            out.append(Instr(Op.JMP_IF_FALSE, 0))
            self._compile_stmt(stmt.body, out, in_function=in_function)
            out.append(Instr(Op.JMP, loop_start))
            out[jmp_false_idx] = Instr(Op.JMP_IF_FALSE, len(out))
            return
        raise AssertionError("unknown stmt")

    def _compile_expr(self, expr: Expr, out: List[Instr]) -> None:
        if isinstance(expr, IntLit):
            out.append(Instr(Op.CONST_I64, expr.value))
            return
        if isinstance(expr, StringLit):
            out.append(Instr(Op.CONST_STRING, self._intern(expr.value)))
            return
        if isinstance(expr, BoolLit):
            out.append(Instr(Op.CONST_BOOL, 1 if expr.value else 0))
            return
        if isinstance(expr, VarRef):
            slot, is_local = self._resolve_var(expr.name, expr.span)
            out.append(Instr(Op.LOAD if is_local else Op.LOAD_UPVALUE, slot))
            return
        if isinstance(expr, CallExpr):
            fn_name = self._resolve_function(expr.callee, expr.span)
            for arg in expr.args:
                self._compile_expr(arg, out)
            if fn_name.startswith("host."):
                out.append(Instr(Op.HOST_CALL, self._intern(fn_name[len("host.") :])))
            else:
                if fn_name not in self.function_ids:
                    raise AxiomCompileError(f"undefined function {fn_name!r}", expr.span)
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

    def _resolve_function(self, fn_name: str, span) -> str:
        if fn_name.startswith("host."):
            return fn_name
        if "." not in fn_name:
            for scope in reversed(self.function_scope_stack):
                if fn_name in scope:
                    return scope[fn_name]
        if fn_name in self.function_ids:
            return fn_name
        raise AxiomCompileError(f"undefined function {fn_name!r}", span)
