# Axiom 🧠⚙️

**Axiom** is a small experimental programming language with:

- 📝 a hand-written lexer and parser
- ✅ a real type checker
- 🧪 a tree-walking interpreter
- 📦 a portable bytecode format
- ⚡ a stack VM
- 🔌 a constrained `host.*` bridge for tool/runtime integration
- 🦀 a new Rust `stage1` native bootstrap under `stage1/`

The project is intentionally small, readable, and standard-library only. The goal is a workable compiler that stays easy to inspect end to end.

## ✨ Current Status

Axiom currently supports:

- typed `let` bindings
- typed function parameters and return values
- scalar types: `int`, `string`, `bool`
- `if` / `while`
- nested functions and lexical closures
- file-based imports
- package manifests and build/run/check/test commands
- bytecode compilation and VM execution
- registry-backed host capabilities

Current bytecode version: **AXBC v0.9**

The repo now has two tracks:

- `stage0`: the current Python implementation in `axiom/`
- `stage1`: a Rust bootstrap compiler in `stage1/` with a tiny native subset

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

# Explore Axiom interactively
python -m axiom repl
# Inside the REPL, use :help for commands and :quit or :exit to leave.

# Compile to bytecode
python -m axiom compile examples/arith.ax -o arith.axb

# Run bytecode on the VM
python -m axiom vm arith.axb

# Run the package example
python -m axiom pkg run examples/typed_package

# Run the two-session collaboration demo
python -m axiom pkg run examples/codex_duo_system

# Run the stage1 native hello-world
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/hello

# Run the stage1 multi-file module example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/modules

# Run the stage1 package test suite
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/modules --json

# Run the stage1 local dependency example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/packages

# Run the stage1 local dependency test suite
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/packages --json

# Run the stage1 package-root workspace example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/workspace

# Run the stage1 package-root workspace test suite
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace --json

# Run the stage1 workspace-only example by selecting a member package
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/workspace_only --package workspace-app

# Run the stage1 workspace-only test suite
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace_only --json

# Run the stage1 capability-gated intrinsic example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/capabilities

# Run the stage1 capability-gated intrinsic tests
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/capabilities --json

# Run the stage1 array example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/arrays

# Run the stage1 borrowed-slice + collection-helper example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/slices

# Run the stage1 borrowed-struct + borrowed-enum example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/borrowed_shapes

# Run the stage1 tuple example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/tuples

# Run the stage1 map example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/maps

# Run the stage1 struct example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/structs

# Run the stage1 payload-enum + match example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/enums

# Run the stage1 Option/Result example
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/outcomes
```

## 🧰 Useful Commands

```bash
# Full test suite
python -m unittest discover -v

# Lint
python -m ruff check .

# Typecheck / compile a source file
python -m axiom check examples/arith.ax
python -m axiom check examples/arith.ax --json
python -m axiom compile examples/compile_demo.ax -o compile_demo.axb --json
python -m axiom vm compile_demo.axb
python -m axiom repl

# Build and run a package
python -m axiom pkg build examples/typed_package
python -m axiom pkg run examples/typed_package
python -m axiom pkg build examples/codex_duo_system
python -m axiom pkg run examples/codex_duo_system

# Project shortcuts
make test
make smoke
make stage1-test
make stage1-smoke

# Run the stage1 package test suite directly
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/modules --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/packages --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace_only --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/capabilities --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace --filter core --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/hello --target "$(rustc -vV | sed -n 's/^host: //p')"
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/workspace_only --package workspace-app --json

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
- `import "path"`
- direct calls like `name(arg)` after importing exported symbols
- host calls like `host.version()` and `host.int.parse("41")`

Runtime rules:

- `+` supports `int + int` and `string + string`
- `-`, `*`, `/`, unary `-`, and ordered comparisons are `int`-only
- `==` and `!=` require matching scalar types and return `bool`
- conditions are `bool`-only
- booleans print as `true` / `false`

See [docs/grammar.md](docs/grammar.md), [docs/kernel.md](docs/kernel.md), and [docs/bytecode.md](docs/bytecode.md) for the precise spec.

For the Rust bootstrap split and the current stage1 status summary, see [docs/stage1.md](docs/stage1.md).

For the detailed agent-facing roadmap to the first workable stage1 compiler, see [docs/stage1-agent-grade-compiler.md](docs/stage1-agent-grade-compiler.md).

## 📁 Repo Map

- `axiom/lexer.py`, `axiom/parser.py`, `axiom/loader.py`: source parsing plus file/import loading
- `axiom/checker.py`: AST -> typed validation
- `axiom/semantic_plan.py`: shared nested-function planning and name resolution
- `axiom/compiler.py`: AST -> bytecode
- `axiom/interpreter.py`: AST execution
- `axiom/bytecode.py`: bytecode encoder/decoder
- `axiom/vm.py`: bytecode execution
- `axiom/cli.py`, `axiom/packaging.py`: CLI orchestration and package workflows
- `stage1/`: Rust bootstrap compiler with `axiom.toml`/`axiom.lock` and native bootstrap examples
- `tests/programs/*.ax`: language fixtures
- `examples/typed_package/`: small typed package example
- `examples/codex_duo_system/`: package demo where two imported modules assemble one system

## 🎯 When To Use Axiom

Axiom is a good fit for:

- deterministic automation logic and data transformation
- portable compile-once, run-anywhere bytecode tools
- constrained agents and workers that benefit from capability gating
- readable, auditable runtimes where the full implementation should stay inspectable

Axiom is not currently designed for:

- systems orchestration that shells out to `brew`, `git`, `defaults`, `systemctl`, or similar tools
- dotfile managers, bootstrap scripts, or configuration-management style CLIs
- host-bridge-heavy tools that need file mutation primitives such as write, copy, mkdir, or symlink
- large application frameworks with async runtimes, generics-heavy APIs, FFI, or direct native backends

Today the host bridge is intentionally constrained. Stage0 exposes a small registry of capability-aware host calls and stage1 mirrors that model with compiler-known capability-gated intrinsics plus thin `std/*` wrappers. Stage1 read-only filesystem access is scoped to the package root by default, or to `[capabilities] fs_root = "<relative package path>"` when configured; reads canonicalize the requested path and deny traversal, symlink escapes, and files larger than 64 MiB. Outbound network resolution/HTTP, environment reads, clock access, hashing, and coarse process status checks are also in scope. Full subprocess execution and filesystem mutation are not.

## 🛣 Roadmap

The current stage0 roadmap is still **Phase 9A**:

- 📚 arrays and collections
- 🧩 better package/module ergonomics
- 🔍 richer diagnostics on the typed core

High-level roadmap: [docs/roadmap.md](docs/roadmap.md)

## 🤝 Notes

- The repo uses a project-local `.venv`.
- The host bridge is intentionally constrained and side effects are gated.
- This codebase is optimized for clarity over abstraction density.
