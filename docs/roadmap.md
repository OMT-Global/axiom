# Roadmap (high-level)

This file tracks the Python `stage0` language/runtime line. The Rust `stage1`
bootstrap compiler now lives in `stage1/` and is described in `docs/stage1.md`.

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

## Phase 8 ✅
- Explicit types on `let`, function params, and function returns
- Real `bool`, bool-only control flow, and full type checking before codegen
- Mixed `int | string | bool` runtime values and bytecode `v0.9`

## Phase 9 ✅
- **9A** Arrays: `int[]`, `string[]`, `bool[]` types; array literals `[1, 2, 3]`;
  index expressions `xs[i]`; `host.array.len`; bytecode `v0.10` (`MAKE_ARRAY`,
  `LOAD_INDEX`)
- **9B** Functional array mutation: `host.array.push_int/string/bool` and
  `host.array.set_int/string/bool` builtins (return new lists, no aliasing bugs)
- **9C** First-class function values: `fn(T,...):R` type syntax; `let f: fn(int):int = fact`
  bindings; indirect calls `f(x)`; passing functions as arguments; bytecode `v0.11`
  (`LOAD_FN`, `CALL_INDIRECT`)
- **9D** Multi-error diagnostics: checker collects all statement-level errors in one
  pass and raises `MultiAxiomError`; single-error path still raises `AxiomCompileError`
  for backward compatibility

## Phase 10 ✅
- **10A** Type inference for `let`: omit the annotation and the checker infers the
  type from the RHS expression; empty array literals still require an annotation
- **10B** `for` loop over arrays: `for x in arr { }` iterates over every element;
  compiles to an indexed while-loop using three anonymous temp slots; fully type-checked
- **10C** New host builtins: `host.string.len/concat/contains/starts_with/ends_with/slice/to_int`
  and `host.math.min/max/pow`

## Phase 11 (next)
- Richer diagnostics: span-accurate notes, "did you mean?" suggestions
- Package and module ergonomics for larger multi-file programs
- Stage1 AG1 agent-grade compiler bootstrap
