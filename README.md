# Axiom (VM-first bootstrap skeleton, no LLVM)

This is a **drop-in repo skeleton** to start building a new language called **Axiom** using the VM-first bootstrap path (Option A):

- **Stage0**: reference interpreter (correctness + fast iteration)
- **Stage1**: compiler to **portable bytecode** + a small **stack VM**
- **Stage2**: functions + call/return and call frames in Axiom
- **Stage3**: host-bridge calls (`host.*`) for deterministic tool interop
- **Stage4**: host capability registry and stable host tool namespace (`host.abs`)
- **Phase5**: package/build tooling scaffold

This repo is intentionally small and test-driven. Everything is **standard-library only** (no deps).

## Quickstart

```bash
# Run via interpreter (stage0)
python -m axiom interp examples/arith.ax

# Compile to bytecode (stage1)
python -m axiom compile examples/arith.ax -o /tmp/arith.axb

# Run bytecode on the VM (stage1)
python -m axiom vm /tmp/arith.axb

# Run package main from manifest (stage3 planning)
python -m axiom pkg run .

# Inspect host bridge capabilities for tooling
python -m axiom host list

# Run conformance tests (interpreter vs VM + expected output)
python -m unittest discover -v

# Create package scaffold and build output artifact
python -m axiom pkg init .
python -m axiom pkg build .
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
- variables
- function calls: `name(arg, ...)`
- `import "path"` for file-level module loading (resolved relative to importing file)
- host bridge calls: `host.version()`, `host.print(value)`, `host.read(prompt)`, `host.abs(value)`, `host.math.abs(value)` (gated side effects apply to `print`/`read` only)
- host bridge calls are registry-backed, and new `host.*` functions can be added with
  `axiom.host.register_host_builtin(name, arity, side_effecting, handler)`.
- `+ - * /` with parentheses
- comparisons: `== != < <= > >=` (results are `0` or `1`)
- unary `-`

Runtime semantics are int-only; `0` is false and non-zero is true.

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

- Add types beyond `i64`-like ints
- Add functions + call frames
- Add module/package system + tooling (formatter/LSP)
- Add a real error reporter (spans -> line/col snippets)

See `docs/roadmap.md`.
