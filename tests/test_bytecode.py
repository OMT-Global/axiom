from __future__ import annotations

import struct
import unittest
from unittest.mock import patch

import axiom.bytecode as bytecode_module
from axiom.bytecode import (
    MAGIC,
    MAX_INSTRUCTIONS,
    MAX_MODULE_ENTRIES,
    MAX_MODULES,
    MAX_STRINGS,
    VERSION_MAJOR,
    VERSION_MINOR,
    Bytecode,
)
from axiom.errors import AxiomBytecodeError, AxiomCompileError


class BytecodeTests(unittest.TestCase):
    def _header(self) -> bytearray:
        data = bytearray()
        data += MAGIC
        data += struct.pack("<HH", VERSION_MAJOR, VERSION_MINOR)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 0)
        return data

    def test_decode_rejects_oversized_string_length_before_payload_read(self) -> None:
        data = self._header()
        data += struct.pack("<I", 1)
        data += struct.pack("<I", 9)

        with patch.object(bytecode_module, "MAX_STRING_BYTES", 8):
            with self.assertRaises(AxiomBytecodeError) as cm:
                Bytecode.decode(bytes(data))

        self.assertIn("bytecode string exceeds", str(cm.exception))

    def test_decode_rejects_string_count_above_limit(self) -> None:
        data = self._header()
        data += struct.pack("<I", MAX_STRINGS + 1)

        with self.assertRaises(AxiomBytecodeError) as cm:
            Bytecode.decode(bytes(data))

        self.assertIn("string table count", str(cm.exception))

    def test_decode_rejects_string_count_larger_than_remaining_minimum_bytes(self) -> None:
        data = self._header()
        data += struct.pack("<I", 1)

        with self.assertRaises(AxiomBytecodeError) as cm:
            Bytecode.decode(bytes(data))

        self.assertIn("remaining bytecode length", str(cm.exception))

    def test_decode_rejects_instruction_count_above_limit(self) -> None:
        data = self._header()
        data += struct.pack("<I", 0)
        data += struct.pack("<I", MAX_INSTRUCTIONS + 1)

        with self.assertRaises(AxiomBytecodeError) as cm:
            Bytecode.decode(bytes(data))

        self.assertIn("instruction count", str(cm.exception))

    def test_decode_rejects_instruction_count_larger_than_remaining_minimum_bytes(
        self,
    ) -> None:
        data = self._header()
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 1)

        with self.assertRaises(AxiomBytecodeError) as cm:
            Bytecode.decode(bytes(data))

        self.assertIn("remaining bytecode length", str(cm.exception))

    def test_decode_rejects_module_count_above_limit(self) -> None:
        data = self._header()
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", MAX_MODULES + 1)

        with self.assertRaises(AxiomBytecodeError) as cm:
            Bytecode.decode(bytes(data))

        self.assertIn("module count", str(cm.exception))

    def test_decode_rejects_module_count_larger_than_remaining_minimum_bytes(
        self,
    ) -> None:
        data = self._header()
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 1)

        with self.assertRaises(AxiomBytecodeError) as cm:
            Bytecode.decode(bytes(data))

        self.assertIn("remaining bytecode length", str(cm.exception))

    def test_decode_rejects_module_entry_count_above_limit(self) -> None:
        data = self._header()
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 1)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", MAX_MODULE_ENTRIES + 1)

        with self.assertRaises(AxiomBytecodeError) as cm:
            Bytecode.decode(bytes(data))

        self.assertIn("module entry count", str(cm.exception))

    def test_decode_rejects_module_entry_count_larger_than_remaining_minimum_bytes(
        self,
    ) -> None:
        data = self._header()
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 1)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 1)

        with self.assertRaises(AxiomBytecodeError) as cm:
            Bytecode.decode(bytes(data))

        self.assertIn("remaining bytecode length", str(cm.exception))

    def test_encode_rejects_oversized_string(self) -> None:
        oversized = "x" * 9
        bytecode = Bytecode(
            strings=[oversized],
            instructions=[],
            locals_count=0,
            functions=[],
        )

        with patch.object(bytecode_module, "MAX_STRING_BYTES", 8):
            with self.assertRaises(AxiomCompileError) as cm:
                bytecode.encode()

        self.assertIn("bytecode string exceeds", cm.exception.message)


if __name__ == "__main__":
    unittest.main()
