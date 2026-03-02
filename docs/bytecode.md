# Axiom bytecode format (AXBC v0.1)

This project uses a tiny custom binary format (no deps) to keep the bootstrap surface small.

All integers are little-endian.

## File layout

- 4 bytes: magic `AXBC`
- u16: version_major (currently 0)
- u16: version_minor (currently 1)
- u32: locals_count
- u32: string_table_count (N)
- N times:
  - u32: byte_len
  - bytes: UTF-8 string
- u32: instruction_count (M)
- M times:
  - u8: opcode
  - payload (depends on opcode)

## Opcodes

- 0x01 CONST_I64  (i64)
- 0x02 LOAD       (u32 slot)
- 0x03 STORE      (u32 slot)
- 0x04 ADD
- 0x05 SUB
- 0x06 MUL
- 0x07 DIV
- 0x08 PRINT
- 0x09 POP
- 0x0A HALT
