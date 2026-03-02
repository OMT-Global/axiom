from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, TextIO

from .bytecode import Bytecode, Instr, Op
from .errors import AxiomRuntimeError


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
        ins = bytecode.instructions
        while self.ip < len(ins):
            i = ins[self.ip]
            self.ip += 1

            if i.op == Op.CONST_I64:
                self.stack.append(int(i.arg))
                continue

            if i.op == Op.LOAD:
                slot = int(i.arg)
                try:
                    self.stack.append(self.locals[slot])
                except IndexError:
                    raise AxiomRuntimeError(f"bad LOAD slot {slot}")
                continue

            if i.op == Op.STORE:
                slot = int(i.arg)
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on STORE")
                v = self.stack.pop()
                try:
                    self.locals[slot] = v
                except IndexError:
                    raise AxiomRuntimeError(f"bad STORE slot {slot}")
                continue

            if i.op in (Op.ADD, Op.SUB, Op.MUL, Op.DIV):
                b, a = self._pop2()
                if i.op == Op.ADD:
                    self.stack.append(a + b)
                elif i.op == Op.SUB:
                    self.stack.append(a - b)
                elif i.op == Op.MUL:
                    self.stack.append(a * b)
                elif i.op == Op.DIV:
                    if b == 0:
                        raise AxiomRuntimeError("division by zero")
                    self.stack.append(int(a / b))
                continue

            if i.op == Op.PRINT:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on PRINT")
                v = self.stack.pop()
                out.write(f"{v}\n")
                continue

            if i.op == Op.POP:
                if not self.stack:
                    raise AxiomRuntimeError("stack underflow on POP")
                self.stack.pop()
                continue

            if i.op == Op.HALT:
                return

            raise AxiomRuntimeError(f"unknown opcode {i.op}")

        raise AxiomRuntimeError("no HALT encountered")

    def _pop2(self) -> tuple[int, int]:
        if len(self.stack) < 2:
            raise AxiomRuntimeError("stack underflow")
        b = self.stack.pop()
        a = self.stack.pop()
        return int(b), int(a)
