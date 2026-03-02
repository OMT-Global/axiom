from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, TextIO

from .bytecode import Bytecode, Op
from .errors import AxiomRuntimeError
from .intops import trunc_div, to_bool_int


@dataclass
class Vm:
    locals_count: int
    stack: List[int] = field(default_factory=list)
    locals: List[int] = field(default_factory=list)
    ip: int = 0

    def __post_init__(self) -> None:
        if not self.locals:
            self.locals = [0] * int(self.locals_count)

    def run(self, bytecode: Bytecode, out: TextIO) -> None:
        self.ip = 0
        self.stack = []
        ins = bytecode.instructions
        while self.ip < len(ins):
            i = ins[self.ip]
            self.ip += 1

            if i.op == Op.CONST_I64:
                self.stack.append(int(i.arg))
            elif i.op == Op.LOAD:
                slot = int(i.arg)
                if slot >= len(self.locals):
                    raise AxiomRuntimeError(f"bad LOAD slot {slot}")
                self.stack.append(self.locals[slot])
            elif i.op == Op.STORE:
                slot = int(i.arg)
                if slot >= len(self.locals):
                    raise AxiomRuntimeError(f"bad STORE slot {slot}")
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on STORE")
                self.locals[slot] = self.stack.pop()
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
