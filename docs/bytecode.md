# Axiom bytecode format (AXBC v0.6)

This project uses a tiny custom binary format (no deps) to keep the bootstrap surface small.

All integers are little-endian.

## File layout

- 4 bytes: magic `AXBC`
- u16: version_major (currently 0)
- u16: version_minor (currently 6)
- u32: locals_count
- u32: function_count (K)
- K times:
  - u32: function_name_index (string table index)
  - u32: entry_ip (instruction index)
  - u32: arity
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

- 0x01 CONST_I64      (i64)
- 0x02 LOAD           (u32 slot)
- 0x03 STORE          (u32 slot)
- 0x04 ADD
- 0x05 SUB
- 0x06 MUL
- 0x07 DIV
- 0x08 PRINT
- 0x09 POP
- 0x0A HALT
- 0x0B JMP            (u32 instruction index)
- 0x0C JMP_IF_FALSE   (u32 instruction index)
- 0x0D CMP_EQ
- 0x0E CMP_NE
- 0x0F CMP_LT
- 0x10 CMP_LE
- 0x11 CMP_GT
- 0x12 CMP_GE
- 0x13 CALL            (u32 function index)
- 0x14 RET
- 0x15 HOST_CALL       (u32 host name index)

In `v0.6+`, the `HOST_CALL` operand is a string table index for the
host name (for example `abs` in `host.abs`), resolved at runtime through
`axiom.host` registry.

Builtins available at runtime are still assigned by registry order for
API visibility and host APIs, but bytecode stores names so call-site stability
does not depend on numeric host-id ordering across compilation and execution.

Custom host capabilities can be appended by registering via
`axiom.host.register_host_builtin`.
