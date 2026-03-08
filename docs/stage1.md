# Stage1 bootstrap

This repo now has two tracks:

- `stage0`: the current Python implementation in `axiom/`, used as the reference
  parser/checker/interpreter/VM and the conformance oracle for overlapping language behavior.
- `stage1`: the Rust bootstrap compiler in `stage1/`, used to prove the long-term
  native toolchain split without destabilizing stage0.

## Current bootstrap scope

The Rust compiler is intentionally tiny in this first slice:

- `axiom.toml` and `axiom.lock` are the new manifest and lockfile pair.
- Supported source subset is top-level `import`, `pub struct`, `struct`, `pub fn`, `fn`, `let`, `print`, `if` / `else`, `while`, `return`, variables, function calls, named struct types, array types, array literals, array indexing, struct literals, field access, `+` on `int`/`string`, and scalar comparisons.
- The pipeline is already split into syntax -> HIR -> MIR -> native build.
- `axiomc build` emits a native binary by generating a Rust file and invoking `rustc`.
- A bootstrap ownership rule is active: non-`Copy` values move on binding and call boundaries, non-`Copy` field access and non-`Copy` array indexing conservatively move the owning variable, and branch-local moves conservatively propagate after `if`.

This is not the final backend architecture. It is the smallest executable version of the
stage0/stage1 split that can build a native hello-world and carry the 1.0 package model.

## Commands

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/hello --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/hello --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/hello
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- caps stage1/examples/hello --json
```

## Current gaps

The current bootstrap is enough to prove the split and native artifact path, but it is
still far from the stated 1.0 target for service and agent workloads.

### Language surface gaps

- Modules are now limited to package-local path imports plus direct `pub struct` and `pub fn` exports only.
- Structs and arrays exist now, but there are still no tuples, enums, slices, maps, `Option<T>`, or `Result<T, E>`.
- No `match`, no pattern matching, no generic functions or generic types.
- No methods, trait-style interfaces, closures, or async/await.
- Rebinding and shadowing are intentionally rejected today to keep the bootstrap scope small.

### Type and ownership gaps

- Ownership is still bootstrap-grade even though it now covers all non-`Copy` stage1 values and conservatively handles non-`Copy` field access.
- There are no borrows, mutable borrows, lifetime checks, or first-class partial-move analysis for aggregates and function calls.
- No exhaustiveness checking, no typed error propagation, and no control-flow-sensitive ownership diagnostics beyond simple branches.
- No compile-fail corpus yet for the future ownership rules that a Rust-like language actually needs.

### Package and build graph gaps

- `axiom.toml` and `axiom.lock` exist, but dependencies and workspaces are still rejected.
- The current import model is still intentionally small: relative path imports only, direct `pub struct` / `pub fn` exports only, no aliases, no re-exports, and no namespace-qualified calls.
- There is no package registry flow, no version resolution, and no offline lockfile validation beyond the bootstrap lockfile shape.

### Runtime and standard library gaps

- There is no stage1 standard library yet.
- Capability metadata exists, but there is no enforced runtime capability system behind file, network, process, env, clock, or crypto APIs.
- No async runtime, channels, cancellation, timers, or service-grade I/O surface exists.

### Backend and tooling gaps

- Native builds still work by generating Rust and invoking `rustc`; there is no Cranelift backend yet.
- There is no stage1 formatter, test runner, benchmark harness, doc generator, publisher, or LSP server.
- Diagnostics are still bootstrap-grade: useful JSON exists, but span quality, notes, and compile-fail coverage are limited.
- There are no performance targets or regression gates yet.

## Execution plan

The work should stay incremental. Each slice must keep stage0 stable and leave stage1 in a
working state with concrete tests.

### Slice 2 completed: multi-file callable baseline

Done in the current bootstrap:

- Package-local relative imports are supported.
- Imported modules can expose structs and functions through `pub struct` and `pub fn`.
- Project analysis now walks a small recursive module graph and rewrites cross-file calls before type checking.
- Compile-fail coverage now includes missing imports, private import calls, imported top-level statements, circular imports, basic struct misuse, and basic array misuse.

Current proof points:

- `stage1/examples/hello` remains the single-file callable baseline.
- `stage1/examples/modules` is the checked-in multi-file package example.
- `stage1/examples/arrays` is the checked-in array/bootstrap collection example.
- `stage1/examples/structs` is the checked-in structured-data bootstrap example.
- `make stage1-test stage1-smoke` now covers all four examples.

### Slice 3: structured data and branching semantics

Goal: add the minimum useful data model for service code.

- Struct declarations, literals, named struct types, field access, array types, array literals, and array indexing are now in the bootstrap.
- Add tuples, enums, and maps next.
- Add `match` with exhaustiveness checking.
- Add `Option<T>` and `Result<T, E>` as real language-level types.
- Extend comparisons and control-flow typing across structured data where appropriate.

Exit criteria:

- Stage1 can express request/response-style data without encoding everything as strings.
- Compile-fail tests cover bad field access, invalid constructors, and non-exhaustive matches.
- The example set includes one small service-style program using structs or enums.

### Slice 4: ownership and borrowing

Goal: replace the bootstrap move rule with a real Rust-like safety model.

- Generalize moves beyond `string` bindings to non-`Copy` values.
- Add immutable and mutable borrows, lexical lifetime tracking, and aliasing checks.
- Teach the checker about moves through function calls, branches, loops, and aggregate fields.
- Build a dedicated compile-fail corpus for move-after-use, double mutable borrow, mutable-plus-shared borrow, and borrow-outlives-owner errors.

Exit criteria:

- Ownership errors are driven by first-class rules, not bootstrap special cases.
- Borrow-check failures produce stable machine-readable diagnostics.
- Stage1 examples include at least one passing ownership-heavy program and several locked failing cases.

### Slice 5: package graph and capability enforcement

Goal: make stage1 usable for real projects instead of isolated examples.

- Implement dependencies, workspaces, lockfile validation, and package-local module resolution.
- Replace the current placeholder capability view with manifest-driven enforcement in the compiler/runtime boundary.
- Add stable package commands for building and checking multi-package workspaces.
- Add tests for deterministic lockfiles, offline rebuilds, capability-denied calls, and allowed capability flows.

Exit criteria:

- `axiomc check/build/run` work across a small workspace with at least one dependency edge.
- Capability-denied programs fail before native execution.
- `axiom.lock` is part of reproducible builds instead of placeholder metadata.

### Slice 6: standard library and async runtime

Goal: make Axiom plausible for CLI, worker, and service workloads.

- Add the first stage1 standard library modules: `std.io`, `std.fs`, `std.env`, `std.time`, `std.json`, `std.http`, `std.process`, `std.collections`, `std.sync`, and `std.crypto.hash`.
- Add async functions, `await`, task spawning, channels, cancellation, and timeout primitives.
- Connect stdlib operations to capability enforcement instead of implicit host access.
- Add integration tests for file I/O, JSON, HTTP client/server flows, process execution, and async coordination.

Exit criteria:

- Stage1 can implement a small HTTP worker and a queue-style agent task runner.
- Capability-aware stdlib calls are covered by integration tests.
- Async programs build and run without falling back to stage0.

### Slice 7: native backend and toolchain completeness

Goal: move from bootstrap compiler to credible production toolchain.

- Replace generated-Rust codegen with a real native backend, starting with Cranelift AOT.
- Add `axiom test`, `axiom fmt`, `axiom bench`, `axiom doc`, and `axiom publish`.
- Add benchmark gates for CLI startup, JSON throughput, HTTP echo, and worker throughput.
- Improve diagnostics with richer spans, notes, and stable JSON output contracts.

Exit criteria:

- Native binaries come from the stage1 backend directly, not `rustc` on generated Rust.
- Toolchain commands cover the full local loop without relying on stage0.
- Benchmarks establish a tracked baseline against simple Go and Rust reference implementations.

## Working rules for future stage1 work

- Keep `stage0` as the conformance oracle for overlapping features until stage1 owns the full language surface it implements.
- Keep the current dual-track verification gate: `python -m unittest discover -v` for stage0 and `make stage1-test stage1-smoke` for stage1.
- Land stage1 slices in small, reviewable increments; do not combine data-model work, ownership work, and backend replacement in one change.
- Prefer compile-fail tests for language rule changes before broad end-to-end examples.
