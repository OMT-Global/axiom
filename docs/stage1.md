# Stage1 bootstrap

This repo now has two tracks:

- `stage0`: the current Python implementation in `axiom/`, used as the reference
  parser/checker/interpreter/VM and the conformance oracle for overlapping language behavior.
- `stage1`: the Rust bootstrap compiler in `stage1/`, used to prove the long-term
  native toolchain split without destabilizing stage0.

## Current bootstrap scope

The Rust compiler is intentionally small in this bootstrap slice:

- `axiom.toml` and `axiom.lock` are the new manifest and lockfile pair.
- Supported source subset is top-level `import`, `pub struct`, `struct`, `pub enum`, `enum`, `pub fn`, `fn`, `let`, `print`, `if` / `else`, `while`, statement-level `match`, `return`, variables, bare enum variants, tuple-style enum constructors, named-payload enum constructors, payload-binding match arms, named-payload match arms, `Option<T>`, `Result<T, E>`, `Some`, `None`, `Ok`, `Err`, the built-in polymorphic collection helpers `len(...)`, `first(...)`, and `last(...)`, function calls, named struct types, named enum types, tuple types, tuple literals, tuple indexing, map types, map literals, map indexing, array types, array literals, array indexing, borrowed array slice expressions, borrowed slice types, borrowed slices stored inside named structs and enum payloads, borrowed-return aggregates backed by one or more borrowed parameters, struct literals, field access, `+` on `int`/`string`, and scalar comparisons.
- The pipeline is already split into syntax -> HIR -> MIR -> native build.
- `axiomc build` emits a native binary by generating a Rust file and invoking `rustc`.
- A bootstrap ownership rule is active: non-`Copy` values move on binding and call boundaries, non-`Copy` field access, non-`Copy` tuple indexing, non-`Copy` map indexing, and non-`Copy` array indexing conservatively move the owning variable, branch-local moves conservatively propagate after `if` and `match`, statically false `if` / `while` branches are now ignored instead of poisoning later ownership state, and live borrowed slices now block moving their owned collection roots until the borrow scope ends, including when those borrows are wrapped in local tuples, named structs, enum payloads, `Option` / `Result` values, passed through sibling expression evaluation, or introduced by temporary `match` expressions.

This is not the final backend architecture. It is the smallest executable version of the
stage0/stage1 split that can build a native hello-world and carry the 1.0 package model.

## Commands

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/hello --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/hello --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/hello
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/modules --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/packages --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/capabilities --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- caps stage1/examples/hello --json
```

`axiomc test` discovers `src/**/*_test.ax` entrypoints by default, builds each test
as a native artifact, executes it, and compares stdout against a sibling
`*.stdout` golden file when present. Projects that need explicit naming or inline
expectations can still declare `[[tests]]` entries in `axiom.toml`.

## Current gaps

The current bootstrap is enough to prove the split and native artifact path, but it is
still far from the stated 1.0 target for service and agent workloads.

### Language surface gaps

- Modules are now limited to package-local path imports plus direct `pub struct`, `pub enum`, and `pub fn` exports only.
- Structs, tuples, tuple-style enum payloads, named-payload enum variants, `Option<T>`, `Result<T, E>`, maps, arrays, borrowed slice types, borrowed array slice expressions, borrowed slices stored inside named structs and enum payloads, borrowed-return aggregates backed by one or more borrowed parameters, field access, tuple indexing, map indexing, array indexing, exhaustive statement-level `match`, and the built-in collection helpers `len(...)`, `first(...)`, and `last(...)` now exist, but there are still no user-defined generic abstractions or a general borrow system.
- No generic functions or generic types.
- No methods, trait-style interfaces, closures, or async/await.
- Rebinding and shadowing are intentionally rejected today to keep the bootstrap scope small.

### Type and ownership gaps

- Ownership is still bootstrap-grade even though it now covers all non-`Copy` stage1 values, conservatively handles non-`Copy` field access, and enforces immutable live-borrow checks for owned values behind borrowed slices.
- Borrowed slices can now flow through direct `&[T]` returns, named structs, enum payloads, and aggregate return types like `Option<&[T]>` or tuples when they are derived from one or more borrowed parameters, `Option` / `Result` match bindings preserve enough borrow provenance to return those borrowed payloads again, conservative call summaries now keep borrowed-return provenance alive across multiple borrowed parameters, statically false control-flow is now skipped instead of contaminating move state, and live borrowed slices now block later owner moves until their scope ends even when those borrows are stored inside local aggregate wrappers, named structs, enum payloads, or temporary `match` / call expressions, but there are still no general borrows, mutable borrows, lifetime checks, precise path-sensitive borrow narrowing beyond constant conditions, or first-class partial-move analysis for aggregates and function calls.
- Exhaustiveness checking now exists for statement-level enum `match`, but there is still no typed error propagation and no control-flow-sensitive ownership diagnostics beyond simple branches.
- Compile-fail coverage now exists for several bootstrap ownership failures, but there is still no dedicated corpus yet for the broader future borrow rules that a Rust-like language actually needs.

### Package and build graph gaps

- `axiom.toml` and `axiom.lock` now support deterministic local path dependency graphs plus package-root workspace members with relative local paths, but there is still no workspace-only manifest or package-selection flow.
- The current import model is still intentionally small: package-local relative path imports plus dependency-prefixed imports like `core/math.ax`, direct `pub struct` / `pub enum` / `pub fn` exports only, and explicit parser diagnostics for unsupported aliases, re-exports, and namespace-qualified calls.
- There is no package registry flow, no version resolution, and no offline lockfile validation beyond the bootstrap lockfile shape.

### Runtime and standard library gaps

- There is no stage1 standard library yet.
- Capability enforcement now exists for a compiler-known intrinsic slice across all six manifest flags: `fs_read(...)`, `net_resolve(...)`, `process_status(...)`, `env_get(...)`, `clock_now_ms()`, and `crypto_sha256(...)`, but there is still no general stdlib module surface.
- No async runtime, channels, cancellation, timers, or service-grade I/O surface exists.

### Backend and tooling gaps

- Native builds still work by generating Rust and invoking `rustc`; there is no Cranelift backend yet.
- There is no stage1 formatter, benchmark harness, doc generator, publisher, or LSP server yet.
- Diagnostics are still bootstrap-grade: useful JSON exists, but span quality, notes, and compile-fail coverage are limited.
- There are no performance targets or regression gates yet.

## Execution plan

The detailed execution spec for turning stage1 into the first workable compiler now
lives in [docs/stage1-agent-grade-compiler.md](stage1-agent-grade-compiler.md).

Current proof points:

- `stage1/examples/hello` remains the single-file callable baseline.
- `stage1/examples/modules` proves the multi-file package baseline and the new
  `axiomc test` discovery flow.
- `stage1/examples/packages` proves the local path dependency baseline and root-package lockfile validation.
- `stage1/examples/workspace` proves the package-root workspace-member baseline and workspace-aware root lockfile validation.
- `stage1/examples/capabilities` proves the capability-gated fs/net/env/clock/crypto path, while the Rust suite covers the remaining process intrinsic contract.
- `stage1/examples/arrays`, `stage1/examples/maps`, `stage1/examples/tuples`,
  and `stage1/examples/structs` cover the current structured-data floor.
- `stage1/examples/slices`, `stage1/examples/borrowed_shapes`, `stage1/examples/enums`,
  and `stage1/examples/outcomes` cover the current borrow-aware and enum/result floor.
- `make stage1-test stage1-smoke` now covers all thirteen checked-in stage1 examples.

Agent-grade compiler milestone summary:

- `AG0`: freeze the current borrowed-projection baseline as the stage1 entry floor.
- `AG1`: finish ownership and borrowing.
- `AG2`: add the minimum generic abstraction layer.
- `AG3`: add package graph support, stable module rules, and real capability enforcement.
- `AG4`: add the stdlib, async runtime, and HTTP-service-capable runtime surface.
- `AG5`: expand `axiomc test` plus the CLI/worker/service fixtures that close
  the first agent-grade compiler bar.

Important bar definition:

- The first workable-compiler bar is **agent-grade**, not direct-native parity.
- Generated-Rust codegen remains acceptable at that bar as long as the public
  workflow is fully `axiomc`-driven.
- The required proof workloads are a multi-package CLI, a queue-style worker,
  and a small HTTP service.

## Working rules for future stage1 work

- Keep `stage0` as the conformance oracle for overlapping features until stage1 owns the full language surface it implements.
- Keep the current dual-track verification gate: `python -m unittest discover -v` for stage0 and `make stage1-test stage1-smoke` for stage1.
- Land stage1 slices in small, reviewable increments; do not combine data-model work, ownership work, and backend replacement in one change.
- Prefer compile-fail tests for language rule changes before broad end-to-end examples.
