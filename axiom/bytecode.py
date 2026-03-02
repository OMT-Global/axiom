from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional, Tuple
import struct

from .errors import AxiomCompileError

MAGIC = b"AXBC"
VERSION_MAJOR = 0
VERSION_MINOR = 1


class Op:
    CONST_I64 = 0x01
    LOAD = 0x02
    STORE = 0x03
    ADD = 0x04
    SUB = 0x05
    MUL = 0x06
    DIV = 0x07
    PRINT = 0x08
    POP = 0x09
    HALT = 0x0A


@dataclass(frozen=True)
class Instr:
    op: int
    arg: Optional[int] = None  # i64 for CONST, u32 for LOAD/STORE


@dataclass(frozen=True)
class Bytecode:
    strings: List[str]
    instructions: List[Instr]
    locals_count: int

    def encode(self) -> bytes:
        out = bytearray()
        out += MAGIC
        out += struct.pack("<HH", VERSION_MAJOR, VERSION_MINOR)
        out += struct.pack("<I", self.locals_count)

        out += struct.pack("<I", len(self.strings))
        for s in self.strings:
            b = s.encode("utf-8")
            out += struct.pack("<I", len(b))
            out += b

        out += struct.pack("<I", len(self.instructions))
        for ins in self.instructions:
            out += struct.pack("<B", ins.op)
            if ins.op == Op.CONST_I64:
                if ins.arg is None:
                    raise AxiomCompileError("CONST_I64 missing arg")
                out += struct.pack("<q", int(ins.arg))
            elif ins.op in (Op.LOAD, Op.STORE):
                if ins.arg is None:
                    raise AxiomCompileError("LOAD/STORE missing arg")
                out += struct.pack("<I", int(ins.arg))
            else:
                # no payload
                pass

        return bytes(out)

    @staticmethod
    def decode(data: bytes) -> "Bytecode":
        mv = memoryview(data)
        off = 0

        def take(n: int) -> bytes:
            nonlocal off
            if off + n > len(mv):
                raise ValueError("truncated bytecode")
            b = mv[off : off + n].tobytes()
            off += n
            return b

        magic = take(4)
        if magic != MAGIC:
            raise ValueError(f"bad magic: {magic!r}")
        major, minor = struct.unpack("<HH", take(4))
        if major != VERSION_MAJOR:
            raise ValueError(f"unsupported major version {major}")
        if minor > VERSION_MINOR:
            raise ValueError(f"unsupported minor version {minor} (max {VERSION_MINOR})")

        (locals_count,) = struct.unpack("<I", take(4))

        (n_strings,) = struct.unpack("<I", take(4))
        strings: List[str] = []
        for _ in range(n_strings):
            (blen,) = struct.unpack("<I", take(4))
            s = take(blen).decode("utf-8")
            strings.append(s)

        (n_ins,) = struct.unpack("<I", take(4))
        ins: List[Instr] = []
        for _ in range(n_ins):
            (op,) = struct.unpack("<B", take(1))
            if op == Op.CONST_I64:
                (v,) = struct.unpack("<q", take(8))
                ins.append(Instr(op, int(v)))
            elif op in (Op.LOAD, Op.STORE):
                (slot,) = struct.unpack("<I", take(4))
                ins.append(Instr(op, int(slot)))
            else:
                ins.append(Instr(op, None))

        return Bytecode(strings=strings, instructions=ins, locals_count=int(locals_count))
