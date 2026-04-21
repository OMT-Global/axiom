from __future__ import annotations

import struct
import unittest
from unittest.mock import patch

import axiom.bytecode as bytecode_module
from axiom.bytecode import MAGIC, VERSION_MAJOR, VERSION_MINOR, Bytecode
from axiom.errors import AxiomCompileError


class BytecodeTests(unittest.TestCase):
    def test_decode_rejects_oversized_string_length_before_payload_read(self) -> None:
        data = bytearray()
        data += MAGIC
        data += struct.pack("<HH", VERSION_MAJOR, VERSION_MINOR)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 0)
        data += struct.pack("<I", 1)
        data += struct.pack("<I", 9)

        with patch.object(bytecode_module, "MAX_STRING_BYTES", 8):
            with self.assertRaises(ValueError) as cm:
                Bytecode.decode(bytes(data))

        self.assertIn("bytecode string exceeds", str(cm.exception))

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
