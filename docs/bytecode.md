# Axiom bytecode format (AXBC v0.11)

This project uses a tiny custom binary format (no deps) to keep the bootstrap surface small.

All integers are little-endian.

## File layout

- 4 bytes: magic `AXBC`
- u16: version_major (currently 0)
- u16: version_minor (currently 11)
- u32: locals_count
- u32: function_count (K)
- K times:
  - u32: function_name_index (string table index)
  - u32: entry_ip (instruction index)
  - u32: arity
  - u32: locals_count
  - if `version_minor >= 7`:
    - u32: upvalue_count
    - upvalue_count entries:
      - u8: 1 if upvalue is from a local slot, 0 if from an outer upvalue
      - u32: upvalue index (local slot or outer upvalue slot)
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
- 0x16 LOAD_UPVALUE    (u32 upvalue index)
- 0x17 STORE_UPVALUE   (u32 upvalue index)
- 0x18 CLOSE_UPVALUE
- 0x19 CONST_STRING    (u32 string table index)
- 0x1A CONST_BOOL      (u8 0/1)
- 0x1C MAKE_ARRAY      (u32 element count) — pops N elements, pushes array
- 0x1D LOAD_INDEX      — pops index and array, pushes element
- 0x1E LOAD_FN         (u32 function index) — pushes a function value onto the stack
- 0x1F CALL_INDIRECT   (u32 arity) — pops fn value and N args, calls through fn value

In `v0.8+`, string constants are emitted as `CONST_STRING` and resolved through
the bytecode string table.

In `v0.9+`, boolean constants are emitted as `CONST_BOOL`.

In `v0.10+`, arrays are supported: `MAKE_ARRAY` constructs an array from the
top N stack elements; `LOAD_INDEX` indexes into an array.

In `v0.11+`, first-class function values are supported: `LOAD_FN` pushes a
function value by index; `CALL_INDIRECT` calls through a function value.

In `v0.7+`, function metadata may include upvalue descriptors for lexical
captures. During call setup, captured upvalues are bound against the current frame.

In `v0.6+`, the `HOST_CALL` operand is a string table index for the
host name (for example `abs` in `host.abs`), resolved at runtime through
`axiom.host` registry.

For closures:

- `LOAD_UPVALUE` reads from an upvalue cell.
- `STORE_UPVALUE` writes to an upvalue cell.
- `CLOSE_UPVALUE` is reserved for future closure lifecycle support; it is currently a no-op.

Builtins available at runtime are still assigned by registry order for
API visibility and host APIs, but bytecode stores names so call-site stability
does not depend on numeric host-id ordering across compilation and execution.

Custom host capabilities can be appended by registering via
`axiom.host.register_host_builtin`.
