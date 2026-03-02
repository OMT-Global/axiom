# Axiom (VM-first bootstrap skeleton, no LLVM)

This is a **drop-in repo skeleton** to start building a new language called **Axiom** using the VM-first bootstrap path (Option A):

- **Stage0**: reference interpreter (correctness + fast iteration)
- **Stage1**: compiler to **portable bytecode** + a small **stack VM**
- **Stage2**: (later) self-host the compiler in Axiom

This repo is intentionally small and test-driven. Everything is **standard-library only** (no deps).

## Quickstart

```bash
# Run via interpreter (stage0)
python -m axiom interp examples/arith.ax

# Compile to bytecode (stage1)
python -m axiom compile examples/arith.ax -o /tmp/arith.axb

# Run bytecode on the VM (stage1)
python -m axiom vm /tmp/arith.axb

# Run conformance tests (interpreter vs VM + expected output)
python -m unittest discover -v
```

## Axiom v0 language subset

Supported:

- `let name = <expr>`
- `print <expr>`
- integer literals
- variables
- `+ - * /` with parentheses
- unary `-`

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
- Add blocks/scopes + control flow
- Add functions + call frames
- Add module/package system + tooling (formatter/LSP)
- Add a real error reporter (spans -> line/col snippets)

See `docs/roadmap.md`.
