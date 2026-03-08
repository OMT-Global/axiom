# Roadmap (high-level)

## Phase 0 ✅
- Small kernel spec + conformance tests
- Interpreter + bytecode compiler + VM
- Differential testing (interpreter vs VM)

## Phase 1 ✅
- Blocks and scopes
- If/else, while, int-truthy control flow
- Better diagnostics + spans -> line/col snippets

## Phase 2 ✅
- Functions + call frames
- Module system
  - file-based `import` with compile-time module loading

## Phase 3 ✅
- Host bridges for tool interoperability
- Stable bytecode + VM/runtime parity for host calls

## Phase 4 ✅
- Built-in host capability registry for agentic extensibility
- Reserve host namespace for tool calls and reject dotted non-host calls

## Phase 5 ✅
- Package/build tooling (`axiom.pkg`, `axiom pkg init`, `axiom pkg build`)
- Package command coverage (`check`, `host` side-effect gating, CLI checks)

## Phase 6 ✅
- Stable host-tooling contracts for long-running agentic workflows
- Module namespace strategy for future large-language agent compositions

## Phase 7 ✅
- Import-trace diagnostics plus obvious static type-mismatch checks
- Mixed `int | string` runtime values, string literals, and bytecode `v0.8`
- Typed host capability metadata and string-aware host I/O

## Phase 8 (next)
- Explicit types and broader value kinds beyond `int | string`
- Collections and first-class function values
- Package and module ergonomics for larger multi-file programs
