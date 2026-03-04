from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Optional, TextIO

from .bytecode import Bytecode, FunctionMeta, Op
from .errors import AxiomRuntimeError
from .intops import trunc_div, to_bool_int
from .host import HOST_BUILTIN_BY_ID, call_host_builtin_id


@dataclass
class _Frame:
    locals: List[int]
    ret_ip: int


@dataclass
class Vm:
    locals_count: int
    stack: List[int] = field(default_factory=list)
    locals: List[int] = field(default_factory=list)
    functions: Optional[List[FunctionMeta]] = None
    frames: List[_Frame] = field(default_factory=list)
    ip: int = 0
    allow_host_side_effects: bool = False

    def __post_init__(self) -> None:
        if self.locals_count < 0:
            raise ValueError("locals_count must be non-negative")

    def run(self, bytecode: Bytecode, out: TextIO) -> None:
        if self.functions is None:
            self.functions = bytecode.functions

        self.locals = [0] * bytecode.locals_count
        self.stack = []
        self.frames = []
        current_locals = self.locals

        self.ip = 0
        ins = bytecode.instructions
        while self.ip < len(ins):
            i = ins[self.ip]
            self.ip += 1

            if i.op == Op.CONST_I64:
                self.stack.append(int(i.arg))
            elif i.op == Op.LOAD:
                slot = int(i.arg)
                if slot >= len(current_locals):
                    raise AxiomRuntimeError(f"bad LOAD slot {slot}")
                self.stack.append(current_locals[slot])
            elif i.op == Op.STORE:
                slot = int(i.arg)
                if slot >= len(current_locals):
                    raise AxiomRuntimeError(f"bad STORE slot {slot}")
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on STORE")
                current_locals[slot] = self.stack.pop()
            elif i.op in (Op.ADD, Op.SUB, Op.MUL, Op.DIV):
                b, a = self._pop2()
                if i.op == Op.ADD:
                    self.stack.append(a + b)
                elif i.op == Op.SUB:
                    self.stack.append(a - b)
                elif i.op == Op.MUL:
                    self.stack.append(a * b)
                else:
                    if b == 0:
                        raise AxiomRuntimeError("division by zero")
                    self.stack.append(trunc_div(a, b))
            elif i.op in (Op.CMP_EQ, Op.CMP_NE, Op.CMP_LT, Op.CMP_LE, Op.CMP_GT, Op.CMP_GE):
                b, a = self._pop2()
                if i.op == Op.CMP_EQ:
                    self.stack.append(to_bool_int(a == b))
                elif i.op == Op.CMP_NE:
                    self.stack.append(to_bool_int(a != b))
                elif i.op == Op.CMP_LT:
                    self.stack.append(to_bool_int(a < b))
                elif i.op == Op.CMP_LE:
                    self.stack.append(to_bool_int(a <= b))
                elif i.op == Op.CMP_GT:
                    self.stack.append(to_bool_int(a > b))
                else:
                    self.stack.append(to_bool_int(a >= b))
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

                new_locals = [0] * fn.locals_count
                for index, value in enumerate(args):
                    new_locals[index] = value

                self.frames.append(_Frame(locals=current_locals, ret_ip=self.ip))
                current_locals = new_locals
                self.ip = fn.entry
            elif i.op == Op.HOST_CALL:
                if i.arg is None:
                    raise AxiomRuntimeError("host call missing arg")
                host_fn_id = int(i.arg)
                if host_fn_id not in HOST_BUILTIN_BY_ID:
                    raise AxiomRuntimeError(f"invalid host function id {host_fn_id}")
                builtin = HOST_BUILTIN_BY_ID[host_fn_id]
                arg_count = builtin.arity
                side_effectful = builtin.side_effecting
                if len(self.stack) < arg_count:
                    raise AxiomRuntimeError(
                        f"call to host function id {host_fn_id} with {len(self.stack)} values on stack"
                    )
                if side_effectful and not self.allow_host_side_effects:
                    raise AxiomRuntimeError(
                        "host function is side-effecting; enable allow_host_side_effects"
                    )
                args = [self.stack.pop() for _ in range(arg_count)]
                args.reverse()
                result = self._call_host_fn(host_fn_id, args, out)
                self.stack.append(result)
            elif i.op == Op.RET:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on RET")
                if not self.frames:
                    raise AxiomRuntimeError("return outside function")
                result = self.stack.pop()
                frame = self.frames.pop()
                current_locals = frame.locals
                self.ip = frame.ret_ip
                self.stack.append(result)
            elif i.op == Op.JMP:
                self.ip = int(i.arg)
            elif i.op == Op.JMP_IF_FALSE:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on JMP_IF_FALSE")
                cond = self.stack.pop()
                if cond == 0:
                    self.ip = int(i.arg)
            elif i.op == Op.PRINT:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on PRINT")
                out.write(f"{self.stack.pop()}\n")
            elif i.op == Op.POP:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on POP")
                self.stack.pop()
            elif i.op == Op.HALT:
                return
            else:
                raise AxiomRuntimeError(f"unknown opcode {i.op}")

        raise AxiomRuntimeError("no HALT encountered")

    def _pop2(self) -> tuple[int, int]:
        if len(self.stack) < 2:
            raise AxiomRuntimeError("stack underflow")
        b = self.stack.pop()
        a = self.stack.pop()
        return int(b), int(a)

    def _call_host_fn(self, fn_id: int, args: List[int], out: TextIO) -> int:
        if fn_id not in HOST_BUILTIN_BY_ID:
            raise AxiomRuntimeError(f"unknown host function id {fn_id}")
        try:
            return call_host_builtin_id(fn_id, args, out)
        except ValueError as e:
            raise AxiomRuntimeError(str(e)) from e
