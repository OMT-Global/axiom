from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional, TextIO

from .bytecode import Bytecode, FunctionMeta, Op
from .errors import AxiomRuntimeError
from .host import HOST_BUILTINS, HOST_BUILTIN_BY_ID, call_host_builtin, call_host_builtin_id
from .values import (
    FunctionValue,
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
    render_value,
    require_condition_bool,
    sub_values,
)


@dataclass
class _Cell:
    value: Value


@dataclass
class _Frame:
    locals: List[_Cell]
    upvalues: List[_Cell]
    ret_ip: int
    function_index: Optional[int]


@dataclass
class Vm:
    locals_count: int
    stack: List[Value] = field(default_factory=list)
    functions: Optional[List[FunctionMeta]] = None
    frames: List[_Frame] = field(default_factory=list)
    _locals: List[_Cell] = field(default_factory=list)
    _upvalues: List[_Cell] = field(default_factory=list)
    _current_function: Optional[int] = None
    _strings: List[str] = field(default_factory=list)
    _function_name_to_index: Dict[str, int] = field(default_factory=dict)
    ip: int = 0
    allow_host_side_effects: bool = False

    def __post_init__(self) -> None:
        if self.locals_count < 0:
            raise ValueError("locals_count must be non-negative")

    def run(self, bytecode: Bytecode, out: TextIO) -> None:
        if self.functions is None:
            self.functions = bytecode.functions

        self._locals = [_Cell(0) for _ in range(bytecode.locals_count)]
        self._upvalues = []
        self._current_function = None
        self._strings = bytecode.strings
        self.stack = []
        self.frames = []
        self._function_name_to_index = {}
        if self.functions is not None:
            self._function_name_to_index = {
                self._strings[f.name_index]: i for i, f in enumerate(self.functions)
            }
        self.ip = 0
        ins = bytecode.instructions
        while self.ip < len(ins):
            i = ins[self.ip]
            self.ip += 1

            if i.op == Op.CONST_I64:
                self.stack.append(int(i.arg))
            elif i.op == Op.CONST_BOOL:
                if i.arg is None:
                    raise AxiomRuntimeError("bool constant missing arg")
                self.stack.append(bool(i.arg))
            elif i.op == Op.CONST_STRING:
                if i.arg is None:
                    raise AxiomRuntimeError("string constant missing arg")
                index = int(i.arg)
                if index < 0 or index >= len(self._strings):
                    raise AxiomRuntimeError(f"bad CONST_STRING index {index}")
                self.stack.append(self._strings[index])
            elif i.op == Op.LOAD:
                slot = self._to_slot(i.arg)
                self.stack.append(self._current_slot_value(slot, in_upvalue=False))
            elif i.op == Op.STORE:
                slot = self._to_slot(i.arg)
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on STORE")
                self._set_slot_value(slot, self.stack.pop(), in_upvalue=False)
            elif i.op == Op.LOAD_UPVALUE:
                slot = self._to_slot(i.arg)
                self.stack.append(self._current_slot_value(slot, in_upvalue=True))
            elif i.op == Op.STORE_UPVALUE:
                slot = self._to_slot(i.arg)
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on STORE_UPVALUE")
                self._set_slot_value(slot, self.stack.pop(), in_upvalue=True)
            elif i.op in (Op.ADD, Op.SUB, Op.MUL, Op.DIV):
                b, a = self._pop2()
                try:
                    if i.op == Op.ADD:
                        self.stack.append(add_values(a, b, context="operator '+'"))
                    elif i.op == Op.SUB:
                        self.stack.append(sub_values(a, b, context="operator '-'"))
                    elif i.op == Op.MUL:
                        self.stack.append(mul_values(a, b, context="operator '*'"))
                    else:
                        self.stack.append(div_values(a, b, context="operator '/'"))
                except ValueError as e:
                    raise AxiomRuntimeError(str(e)) from e
            elif i.op in (Op.CMP_EQ, Op.CMP_NE, Op.CMP_LT, Op.CMP_LE, Op.CMP_GT, Op.CMP_GE):
                b, a = self._pop2()
                try:
                    if i.op == Op.CMP_EQ:
                        self.stack.append(compare_eq(a, b))
                    elif i.op == Op.CMP_NE:
                        self.stack.append(compare_ne(a, b))
                    elif i.op == Op.CMP_LT:
                        self.stack.append(compare_lt(a, b, context="operator '<'"))
                    elif i.op == Op.CMP_LE:
                        self.stack.append(compare_le(a, b, context="operator '<='"))
                    elif i.op == Op.CMP_GT:
                        self.stack.append(compare_gt(a, b, context="operator '>'"))
                    else:
                        self.stack.append(compare_ge(a, b, context="operator '>='"))
                except ValueError as e:
                    raise AxiomRuntimeError(str(e)) from e
            elif i.op == Op.CALL:
                call_idx = int(i.arg)
                functions = self.functions if self.functions is not None else []
                if call_idx < 0 or call_idx >= len(functions):
                    raise AxiomRuntimeError(f"bad call target {call_idx}")
                fn = functions[call_idx]
                if len(self.stack) < fn.arity:
                    raise AxiomRuntimeError(
                        f"call to {fn.arity}-arg function with {len(self.stack)} values on stack"
                    )

                args = [self.stack.pop() for _ in range(fn.arity)]
                args.reverse()

                new_locals = [_Cell(0) for _ in range(fn.locals_count)]
                for index, value in enumerate(args):
                    new_locals[index].value = value

                new_upvalues = self._build_call_upvalues(call_idx, fn)

                self.frames.append(
                    _Frame(
                        locals=self._locals,
                        upvalues=self._upvalues,
                        ret_ip=self.ip,
                        function_index=self._current_function,
                    )
                )
                self._locals = new_locals
                self._upvalues = new_upvalues
                self._current_function = call_idx
                self._set_ip(fn.entry, len(ins), f"call target {call_idx} entry")
            elif i.op == Op.HOST_CALL:
                if i.arg is None:
                    raise AxiomRuntimeError("host call missing arg")
                host_ref = int(i.arg)
                if bytecode.version_minor < 6:
                    host_fn_name = None
                    host_fn_id = host_ref
                    if host_fn_id not in HOST_BUILTIN_BY_ID:
                        raise AxiomRuntimeError(f"invalid host function id {host_fn_id}")
                    builtin = HOST_BUILTIN_BY_ID[host_fn_id]
                    arg_count = builtin.arity
                    side_effectful = builtin.side_effecting
                else:
                    try:
                        host_fn_name = bytecode.strings[host_ref]
                    except IndexError as e:
                        raise AxiomRuntimeError(f"invalid host function index {host_ref}") from e
                    if host_fn_name not in HOST_BUILTINS:
                        raise AxiomRuntimeError(f"undefined host function {host_fn_name!r}")
                    builtin = HOST_BUILTINS[host_fn_name]
                    arg_count = builtin.arity
                    side_effectful = builtin.side_effecting
                    host_fn_id = None

                if len(self.stack) < arg_count:
                    raise AxiomRuntimeError(
                        f"call to host function with {len(self.stack)} values on stack"
                    )
                if side_effectful and not self.allow_host_side_effects:
                    raise AxiomRuntimeError(
                        "host function is side-effecting; enable allow_host_side_effects"
                    )
                args = [self.stack.pop() for _ in range(arg_count)]
                args.reverse()
                if host_fn_name is None:
                    result = self._call_host_fn(host_fn_id, args, out)
                else:
                    result = self._call_host_fn_name(host_fn_name, args, out)
                self.stack.append(result)
            elif i.op == Op.RET:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on RET")
                if not self.frames:
                    raise AxiomRuntimeError("return outside function")
                result = self.stack.pop()
                frame = self.frames.pop()
                self._locals = frame.locals
                self._upvalues = frame.upvalues
                self._current_function = frame.function_index
                self._set_ip(frame.ret_ip, len(ins), "return address")
                self.stack.append(result)
            elif i.op == Op.JMP:
                self._set_ip(int(i.arg), len(ins), "jump target")
            elif i.op == Op.JMP_IF_FALSE:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on JMP_IF_FALSE")
                cond = self.stack.pop()
                try:
                    if not require_condition_bool(cond):
                        self._set_ip(int(i.arg), len(ins), "conditional jump target")
                except ValueError as e:
                    raise AxiomRuntimeError(str(e)) from e
            elif i.op == Op.PRINT:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on PRINT")
                out.write(f"{render_value(self.stack.pop())}\n")
            elif i.op == Op.POP:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on POP")
                self.stack.pop()
            elif i.op == Op.HALT:
                return
            elif i.op == Op.CLOSE_UPVALUE:
                continue
            elif i.op == Op.MAKE_ARRAY:
                n = int(i.arg)
                if len(self.stack) < n:
                    raise AxiomRuntimeError("stack underflow on MAKE_ARRAY")
                elements = [self.stack.pop() for _ in range(n)]
                elements.reverse()
                self.stack.append(elements)
            elif i.op == Op.LOAD_INDEX:
                if len(self.stack) < 2:
                    raise AxiomRuntimeError("stack underflow on LOAD_INDEX")
                index = self.stack.pop()
                array = self.stack.pop()
                if not isinstance(array, list):
                    raise AxiomRuntimeError("cannot index a non-array value")
                if type(index) is not int:
                    raise AxiomRuntimeError("array index must be int")
                if index < 0 or index >= len(array):
                    raise AxiomRuntimeError(
                        f"array index {index} out of bounds (length {len(array)})"
                    )
                self.stack.append(array[index])
            elif i.op == Op.LOAD_FN:
                fn_id = int(i.arg)
                functions = self.functions if self.functions is not None else []
                if fn_id < 0 or fn_id >= len(functions):
                    raise AxiomRuntimeError(f"bad LOAD_FN index {fn_id}")
                fn_name = self._strings[functions[fn_id].name_index]
                self.stack.append(FunctionValue(fn_name))
            elif i.op == Op.CALL_INDIRECT:
                arity = int(i.arg)
                # Stack layout: [..., arg0, arg1, ..., argN-1, fn_val]
                if len(self.stack) < arity + 1:
                    raise AxiomRuntimeError("stack underflow on CALL_INDIRECT")
                fn_val = self.stack.pop()
                if not isinstance(fn_val, FunctionValue):
                    raise AxiomRuntimeError("CALL_INDIRECT expected a function value on stack")
                fn_idx = self._function_name_to_index.get(fn_val.name)
                if fn_idx is None:
                    raise AxiomRuntimeError(f"undefined function {fn_val.name!r}")
                functions = self.functions if self.functions is not None else []
                fn = functions[fn_idx]
                if arity != fn.arity:
                    raise AxiomRuntimeError(
                        f"indirect call arity mismatch: expected {fn.arity}, got {arity}"
                    )
                args = [self.stack.pop() for _ in range(arity)]
                args.reverse()
                new_locals = [_Cell(0) for _ in range(fn.locals_count)]
                for idx, value in enumerate(args):
                    new_locals[idx].value = value
                new_upvalues = self._build_call_upvalues(fn_idx, fn)
                self.frames.append(
                    _Frame(
                        locals=self._locals,
                        upvalues=self._upvalues,
                        ret_ip=self.ip,
                        function_index=self._current_function,
                    )
                )
                self._locals = new_locals
                self._upvalues = new_upvalues
                self._current_function = fn_idx
                self._set_ip(
                    fn.entry,
                    len(ins),
                    f"indirect call target {fn_idx} entry",
                )
            else:
                raise AxiomRuntimeError(f"unknown opcode {i.op}")

        raise AxiomRuntimeError("no HALT encountered")

    def _pop2(self) -> tuple[Value, Value]:
        if len(self.stack) < 2:
            raise AxiomRuntimeError("stack underflow")
        b = self.stack.pop()
        a = self.stack.pop()
        return b, a

    def _set_ip(self, target: int, instruction_count: int, context: str) -> None:
        if target < 0 or target >= instruction_count:
            raise AxiomRuntimeError(
                f"{context} {target} out of bounds "
                f"(instruction count {instruction_count})"
            )
        self.ip = target

    def _call_host_fn(self, fn_id: int, args: List[Value], out: TextIO) -> Value:
        if fn_id not in HOST_BUILTIN_BY_ID:
            raise AxiomRuntimeError(f"unknown host function id {fn_id}")
        try:
            return call_host_builtin_id(fn_id, args, out)
        except ValueError as e:
            raise AxiomRuntimeError(str(e)) from e

    def _call_host_fn_name(self, name: str, args: List[Value], out: TextIO) -> Value:
        try:
            return call_host_builtin(name, args, out)
        except ValueError as e:
            raise AxiomRuntimeError(str(e)) from e

    def _current_slot_value(self, slot: int, *, in_upvalue: bool) -> Value:
        if in_upvalue:
            if slot >= len(self._upvalues):
                raise AxiomRuntimeError(f"bad UPVALUE slot {slot}")
            return self._upvalues[slot].value
        if slot >= len(self._locals):
            raise AxiomRuntimeError(f"bad LOAD slot {slot}")
        return self._locals[slot].value

    def _set_slot_value(self, slot: int, value: Value, *, in_upvalue: bool) -> None:
        if in_upvalue:
            if slot >= len(self._upvalues):
                raise AxiomRuntimeError(f"bad UPVALUE slot {slot}")
            self._upvalues[slot].value = value
            return
        if slot >= len(self._locals):
            raise AxiomRuntimeError(f"bad STORE slot {slot}")
        self._locals[slot].value = value

    def _to_slot(self, raw_slot: Optional[int]) -> int:
        if raw_slot is None:
            raise AxiomRuntimeError("missing slot")
        return int(raw_slot)

    def _build_call_upvalues(self, fn_index: int, fn: FunctionMeta) -> List[_Cell]:
        if not fn.upvalues:
            return []
        parent_fn_index = self._parent_function_index(fn_index)
        parent_locals, parent_upvalues = self._find_frame_context(parent_fn_index)
        upvalues: List[_Cell] = []
        for upvalue in fn.upvalues:
            if upvalue.from_local:
                if upvalue.index >= len(parent_locals):
                    raise AxiomRuntimeError(f"bad function upvalue slot {upvalue.index}")
                upvalues.append(parent_locals[upvalue.index])
            else:
                if upvalue.index >= len(parent_upvalues):
                    raise AxiomRuntimeError(f"bad function upvalue index {upvalue.index}")
                upvalues.append(parent_upvalues[upvalue.index])
        return upvalues

    def _find_frame_context(
        self, function_index: Optional[int]
    ) -> tuple[List[_Cell], List[_Cell]]:
        if function_index is None:
            return (self._locals, self._upvalues)
        if self._current_function == function_index:
            return (self._locals, self._upvalues)
        for frame in reversed(self.frames):
            if frame.function_index == function_index:
                return (frame.locals, frame.upvalues)
        raise AxiomRuntimeError(
            f"closure parent frame {function_index} not found during call"
        )

    def _parent_function_index(self, fn_index: int) -> Optional[int]:
        if self.functions is None:
            return None
        if fn_index < 0 or fn_index >= len(self.functions):
            raise AxiomRuntimeError(f"bad function index {fn_index}")
        fn_name_index = self.functions[fn_index].name_index
        if fn_name_index < 0 or fn_name_index >= len(self._strings):
            raise AxiomRuntimeError(f"bad function name index {fn_name_index}")
        qualified_name = self._strings[fn_name_index]
        if "." not in qualified_name:
            return None
        parent_name = qualified_name.rsplit(".", 1)[0]
        parent_index = self._function_name_to_index.get(parent_name)
        if parent_index is None:
            raise AxiomRuntimeError(f"bad parent function {parent_name!r} for {qualified_name!r}")
        return parent_index
