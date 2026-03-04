# Roadmap (high-level)

## Phase 0 (this repo)
- Small kernel spec + conformance tests
- Interpreter + bytecode compiler + VM
- Differential testing (interpreter vs VM)

## Phase 1
- Blocks and scopes
- If/else, while, boolean type
- Better diagnostics + spans -> line/col snippets

## Phase 2
- Functions + call frames
- Module system
  - file-based `import` (prototype: compile-time inlining)

## Phase 3
- Host bridges for tool interoperability
- Stable bytecode + VM/runtime parity for host calls

## Phase 4
- Built-in host capability registry for agentic extensibility

## Phase 5
- Package/build tooling skeleton (`axiom.pkg`, `axiom pkg init`, `axiom pkg build`)
