# Roadmap (high-level)

## Phase 0 ✅
- Small kernel spec + conformance tests
- Interpreter + bytecode compiler + VM
- Differential testing (interpreter vs VM)

## Phase 1 ✅
- Blocks and scopes
- If/else, while, boolean type
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

## Phase 7 (next)
- Richer diagnostics and developer tooling beyond single-span error snippets
- Types beyond int-only runtime semantics
- Package and module ergonomics for larger multi-file programs
