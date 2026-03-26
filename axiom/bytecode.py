from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Optional
import struct

from .errors import AxiomCompileError

MAGIC = b"AXBC"
VERSION_MAJOR = 0
VERSION_MINOR = 10


class Op:
    CONST_I64 = 0x01
    CONST_BOOL = 0x1A
    CONST_STRING = 0x19
    LOAD = 0x02
    STORE = 0x03
    ADD = 0x04
    SUB = 0x05
    MUL = 0x06
    DIV = 0x07
    PRINT = 0x08
    POP = 0x09
    HALT = 0x0A
    JMP = 0x0B
    JMP_IF_FALSE = 0x0C
    CMP_EQ = 0x0D
    CMP_NE = 0x0E
    CMP_LT = 0x0F
    CMP_LE = 0x10
    CMP_GT = 0x11
    CMP_GE = 0x12
    CALL = 0x13
    RET = 0x14
    HOST_CALL = 0x15
    LOAD_UPVALUE = 0x16
    STORE_UPVALUE = 0x17
    CLOSE_UPVALUE = 0x18
    MAKE_ARRAY = 0x1C
    LOAD_INDEX = 0x1D


@dataclass(frozen=True)
class Upvalue:
    from_local: bool
    index: int


@dataclass(frozen=True)
class FunctionMeta:
    name_index: int
    entry: int
    arity: int
    locals_count: int
    upvalues: List[Upvalue] = field(default_factory=list)


@dataclass(frozen=True)
class ModuleMeta:
    namespace_index: int
    function_indices: List[int] = field(default_factory=list)


@dataclass(frozen=True)
class Instr:
    op: int
    arg: Optional[int] = None


@dataclass(frozen=True)
class Bytecode:
    strings: List[str]
    instructions: List[Instr]
    locals_count: int
    functions: List[FunctionMeta]
    modules: List[ModuleMeta] = field(default_factory=list)
    version_minor: int = VERSION_MINOR

    def encode(self) -> bytes:
        out = bytearray()
        out += MAGIC
        out += struct.pack("<HH", VERSION_MAJOR, self.version_minor)
        out += struct.pack("<I", self.locals_count)

        out += struct.pack("<I", len(self.functions))
        for f in self.functions:
            out += struct.pack("<I", f.name_index)
            out += struct.pack("<I", f.entry)
            out += struct.pack("<I", f.arity)
            out += struct.pack("<I", f.locals_count)
            if self.version_minor >= 7:
                out += struct.pack("<I", len(f.upvalues))
                for upvalue in f.upvalues:
                    out += struct.pack("<B", 1 if upvalue.from_local else 0)
                    out += struct.pack("<I", upvalue.index)

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
            elif ins.op == Op.CONST_BOOL:
                if ins.arg is None:
                    raise AxiomCompileError("CONST_BOOL missing arg")
                if self.version_minor < 9:
                    raise AxiomCompileError("CONST_BOOL requires bytecode version 0.9+")
                out += struct.pack("<B", 1 if int(ins.arg) else 0)
            elif ins.op == Op.CONST_STRING:
                if ins.arg is None:
                    raise AxiomCompileError("CONST_STRING missing arg")
                if self.version_minor < 8:
                    raise AxiomCompileError("CONST_STRING requires bytecode version 0.8+")
                out += struct.pack("<I", int(ins.arg))
            elif ins.op == Op.MAKE_ARRAY:
                if ins.arg is None:
                    raise AxiomCompileError("MAKE_ARRAY missing arg")
                out += struct.pack("<I", int(ins.arg))
            elif ins.op in (
                Op.LOAD,
                Op.STORE,
                Op.JMP,
                Op.JMP_IF_FALSE,
                Op.CALL,
                Op.HOST_CALL,
                Op.LOAD_UPVALUE,
                Op.STORE_UPVALUE,
            ):
                if ins.arg is None:
                    raise AxiomCompileError("opcode missing arg")
                out += struct.pack("<I", int(ins.arg))

        out += struct.pack("<I", len(self.modules))
        if self.version_minor >= 7:
            for module in self.modules:
                out += struct.pack("<I", module.namespace_index)
                out += struct.pack("<I", len(module.function_indices))
                for function_index in module.function_indices:
                    out += struct.pack("<I", int(function_index))

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

        if take(4) != MAGIC:
            raise ValueError("bad magic")
        major, minor = struct.unpack("<HH", take(4))
        if major != VERSION_MAJOR:
            raise ValueError(f"unsupported major version {major}")
        if minor > VERSION_MINOR:
            raise ValueError(f"unsupported minor version {minor} (max {VERSION_MINOR})")

        (locals_count,) = struct.unpack("<I", take(4))

        (n_functions,) = struct.unpack("<I", take(4))
        functions: List[FunctionMeta] = []
        for _ in range(n_functions):
            (name_index,) = struct.unpack("<I", take(4))
            (entry,) = struct.unpack("<I", take(4))
            (arity,) = struct.unpack("<I", take(4))
            (func_locals_count,) = struct.unpack("<I", take(4))
            upvalues: List[Upvalue] = []
            if minor >= 7:
                (upvalue_count,) = struct.unpack("<I", take(4))
                for _ in range(upvalue_count):
                    (from_local,) = struct.unpack("<B", take(1))
                    (index,) = struct.unpack("<I", take(4))
                    upvalues.append(Upvalue(bool(from_local), int(index)))
            functions.append(
                FunctionMeta(
                    name_index=int(name_index),
                    entry=int(entry),
                    arity=int(arity),
                    locals_count=int(func_locals_count),
                    upvalues=upvalues,
                )
            )

        (n_strings,) = struct.unpack("<I", take(4))
        strings: List[str] = []
        for _ in range(n_strings):
            (blen,) = struct.unpack("<I", take(4))
            strings.append(take(blen).decode("utf-8"))

        (n_ins,) = struct.unpack("<I", take(4))
        ins: List[Instr] = []
        for _ in range(n_ins):
            (op,) = struct.unpack("<B", take(1))
            if op == Op.CONST_I64:
                (v,) = struct.unpack("<q", take(8))
                ins.append(Instr(op, int(v)))
            elif op == Op.CONST_BOOL:
                if minor < 9:
                    raise ValueError("CONST_BOOL requires bytecode version 0.9+")
                (v,) = struct.unpack("<B", take(1))
                ins.append(Instr(op, int(v)))
            elif op == Op.CONST_STRING:
                if minor < 8:
                    raise ValueError("CONST_STRING requires bytecode version 0.8+")
                (index,) = struct.unpack("<I", take(4))
                ins.append(Instr(op, int(index)))
            elif op == Op.MAKE_ARRAY:
                (n,) = struct.unpack("<I", take(4))
                ins.append(Instr(op, int(n)))
            elif op in (
                Op.LOAD,
                Op.STORE,
                Op.JMP,
                Op.JMP_IF_FALSE,
                Op.CALL,
                Op.HOST_CALL,
                Op.LOAD_UPVALUE,
                Op.STORE_UPVALUE,
            ):
                (slot,) = struct.unpack("<I", take(4))
                ins.append(Instr(op, int(slot)))
            else:
                ins.append(Instr(op, None))

        modules: List[ModuleMeta] = []
        if minor >= 7 and off < len(mv):
            (n_modules,) = struct.unpack("<I", take(4))
            for _ in range(n_modules):
                (namespace_index,) = struct.unpack("<I", take(4))
                (n_entries,) = struct.unpack("<I", take(4))
                function_indices: List[int] = []
                for _ in range(n_entries):
                    (function_index,) = struct.unpack("<I", take(4))
                    function_indices.append(int(function_index))
                modules.append(
                    ModuleMeta(
                        namespace_index=int(namespace_index),
                        function_indices=function_indices,
                    )
                )

        return Bytecode(
            strings=strings,
            instructions=ins,
            locals_count=int(locals_count),
            functions=functions,
            modules=modules,
            version_minor=minor,
        )
