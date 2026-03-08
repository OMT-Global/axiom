# Axiom 🧠⚙️

**Axiom** is a small experimental programming language with:

- 📝 a hand-written lexer and parser
- ✅ a real type checker
- 🧪 a tree-walking interpreter
- 📦 a portable bytecode format
- ⚡ a stack VM
- 🔌 a constrained `host.*` bridge for tool/runtime integration

The project is intentionally small, readable, and standard-library only. The goal is a workable compiler that stays easy to inspect end to end.

## ✨ Current Status

Axiom currently supports:

- typed `let` bindings
- typed function parameters and return values
- scalar types: `int`, `string`, `bool`
- `if` / `while`
- nested functions and lexical closures
- file-based imports
- package manifests and build/run/check commands
- bytecode compilation and VM execution
- registry-backed host capabilities

Current bytecode version: **AXBC v0.9**

## 👀 Example

```axiom
fn greet(name: string): string {
  return "hello, " + name
}

let ready: bool = true

if ready {
  print greet("axiom")
}
```

## 🚀 Quickstart

```bash
# Clone
git clone https://github.com/OMT-Global/axiom.git
cd axiom

# Create the local virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Run a program through the interpreter
python -m axiom interp examples/arith.ax

# Compile to bytecode
python -m axiom compile examples/arith.ax -o /tmp/arith.axb

# Run bytecode on the VM
python -m axiom vm /tmp/arith.axb

# Run the package example
python -m axiom pkg run examples/typed_package
```

## 🧰 Useful Commands

```bash
# Full test suite
python -m unittest discover -v

# Typecheck / compile a source file
python -m axiom check examples/arith.ax

# Build and run a package
python -m axiom pkg build examples/typed_package
python -m axiom pkg run examples/typed_package

# Inspect host capabilities
python -m axiom host list
python -m axiom host describe
```

## 🗣 Language Snapshot

Supported syntax today:

- `let name: type = expr`
- `name = expr`
- `print expr`
- `fn name(arg: type, ...): type { ... }`
- `if expr { ... } else { ... }`
- `while expr { ... }`
- `import "path"` and `import "path" as alias`
- calls like `name(arg)` and `module.name(arg)`
- host calls like `host.version()` and `host.int.parse("41")`

Runtime rules:

- `+` supports `int + int` and `string + string`
- `-`, `*`, `/`, unary `-`, and ordered comparisons are `int`-only
- `==` and `!=` require matching scalar types and return `bool`
- conditions are `bool`-only
- booleans print as `true` / `false`

See [docs/grammar.md](docs/grammar.md), [docs/kernel.md](docs/kernel.md), and [docs/bytecode.md](docs/bytecode.md) for the precise spec.

## 📁 Repo Map

- `axiom/lexer.py`, `axiom/parser.py`: source -> AST
- `axiom/checker.py`: AST -> typed validation
- `axiom/compiler.py`: AST -> bytecode
- `axiom/interpreter.py`: AST execution
- `axiom/bytecode.py`: bytecode encoder/decoder
- `axiom/vm.py`: bytecode execution
- `tests/programs/*.ax`: language fixtures
- `examples/typed_package/`: small typed package example

## 🛣 Roadmap

The next major step is **Phase 9A**:

- 📚 arrays and collections
- 🧩 better package/module ergonomics
- 🔍 richer diagnostics on the typed core

High-level roadmap: [docs/roadmap.md](docs/roadmap.md)

## 🤝 Notes

- The repo uses a project-local `.venv`.
- The host bridge is intentionally constrained and side effects are gated.
- This codebase is optimized for clarity over abstraction density.
