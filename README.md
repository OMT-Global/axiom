# Axiom (VM-first bootstrap runtime and tooling, no LLVM)

This repo now covers the VM-first bootstrap path through package tooling and
host contracts:

- **Phase 0**: reference interpreter, portable bytecode compiler, stack VM, and conformance tests
- **Phase 1**: blocks/scopes, control flow, and span-aware diagnostics
- **Phase 2**: functions, call frames, lexical closures, and file-based modules
- **Phase 3**: deterministic `host.*` bridge calls with interpreter/VM parity
- **Phase 4**: registry-backed host capabilities and reserved host namespace rules
- **Phase 5**: package manifest, build helpers, and CLI package commands
- **Phase 6**: stable host contract metadata and package-level host contract checks
- **Phase 7**: mixed `int | string` values, bytecode `v0.8`, and typed host capability metadata

This repo is intentionally small and test-driven. Everything is **standard-library only** (no deps).

## Quickstart

```bash
# Run via interpreter (stage0)
python -m axiom interp examples/arith.ax

# Compile to bytecode (stage1)
python -m axiom compile examples/arith.ax -o /tmp/arith.axb

# Run bytecode on the VM (stage1)
python -m axiom vm /tmp/arith.axb

# Run package main from manifest
python -m axiom pkg run .

# Inspect host bridge capabilities for tooling
python -m axiom host list
# Emit compact machine-readable output for scripts
python -m axiom host list --compact
# Inspect the full host contract (schema + runtime version + capabilities) for tooling
python -m axiom host describe
# Inspect only deterministic host calls for agentic tooling
python -m axiom host list --safe-only

# Run conformance tests (interpreter vs VM + expected output)
python -m unittest discover -v

# Create package scaffold and build output artifact
python -m axiom pkg init .
python -m axiom pkg build .
python -m axiom pkg check .
```

See `docs/package.md` for manifest format and build behavior.

## Axiom v0 language subset

Supported:

- `let name = <expr>`
- `name = <expr>` assignment (nearest lexical binding)
- `print <expr>`
- block scopes:
  - `{ ... }` introduces nested lexical scope
- control flow:
  - `if <expr> { ... } else { ... }`
  - `while <expr> { ... }`
- integer literals
- string literals
- variables
- identifiers named `host` are reserved (`let host`, function parameters, and function names)
- function calls: `name(arg, ...)`
- `import "path"` (or `import "path" as alias`) for file-level module loading (resolved relative to importing file)
  - Import paths must be relative and may not use parent traversal (`..`).
- host bridge calls: `host.version()`, `host.print(value)`, `host.read(prompt)`, `host.int.parse(text)`, `host.abs(value)`, `host.math.abs(value)` (gated side effects apply to `print`/`read` only)
- deterministic host calls are available without runtime flags; side-effecting host calls require the explicit `--allow-host-side-effects` option
- host bridge calls are registry-backed, and new `host.*` functions can be added with
  `axiom.host.register_host_builtin(name, arity, side_effecting, handler, arg_kinds=..., return_kind=...)`.
- `+ - * /` with parentheses
- comparisons: `== != < <= > >=` (results are `0` or `1`)
- unary `-`

Runtime semantics are `int | string`. Conditions remain int-only: `0` is false and
non-zero is true. `+` supports `int+int` and `string+string`; ordered comparisons
and arithmetic other than `+` remain int-only.

Statements can end with `;` or a newline.

See `docs/grammar.md`.

## Project layout

- `axiom/lexer.py` / `axiom/parser.py`: parse Axiom into an AST
- `axiom/interpreter.py`: execute AST directly (stage0)
- `axiom/compiler.py`: compile AST -> bytecode (stage1)
- `axiom/bytecode.py`: bytecode format + encoder/decoder
- `axiom/vm.py`: bytecode VM (stage1)
- `tests/programs/*.ax`: conformance programs
- `tests/programs/*.out`: expected stdout for each program

## Next steps

- Add explicit types and broader value kinds beyond `int | string`
- Extend diagnostics beyond single-span snippets (for example import traces and richer runtime context)
- Continue host-native package tooling hardening
- Improve collections and module/package ergonomics for larger multi-file programs

See `docs/roadmap.md`.
