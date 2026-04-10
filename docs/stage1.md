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
- Stage1 now ships a synthetic standard library surface under the `std/` import prefix with eight landed modules. Six are thin wrappers over single-intrinsic capability-gated surfaces, one per capability class: `std/time.ax` exposes `now_ms(): int` on top of `clock_now_ms`, `std/env.ax` exposes `get_env(key: string): Option<string>` on top of `env_get`, `std/fs.ax` exposes `read_file(path: string): Option<string>` on top of `fs_read`, `std/net.ax` exposes `resolve(host: string): Option<string>` on top of `net_resolve`, `std/process.ax` exposes `run_status(command: string): int` on top of `process_status`, and `std/crypto_hash.ax` (the stage1 spelling of `std.crypto.hash`) exposes `sha256(input: string): string` on top of `crypto_sha256`. Each of those six requires the importing package to declare the matching capability flag (`clock`, `env`, `fs`, `net`, `process`, or `crypto`). The seventh module, `std/http.ax`, shares the `net` capability surface with `std/net.ax` and exposes `get(url: string): Option<string>` on top of a new `http_get` intrinsic that implements a blocking HTTP/1.0 client over raw TCP (http:// only, HTTPS/TLS land in a follow-on slice). The eighth module, `std/io.ax`, is the first stdlib surface not tied to a capability flag: it exposes `eprintln(text: string): int` on top of a new ungated `io_eprintln` intrinsic that writes a line to stderr and returns the number of bytes written (`-1` on error), matching the ambient status of the `print` statement.
- The pipeline is already split into syntax -> HIR -> MIR -> native build.
- `axiomc build` emits a native binary by generating a Rust file and invoking `rustc`.
- A bootstrap ownership rule is active: non-`Copy` values move on binding and call boundaries, non-`Copy` field access, non-`Copy` tuple indexing, non-`Copy` map indexing, and non-`Copy` array indexing conservatively move the owning variable, branch-local moves conservatively propagate after `if` and `match`, statically false `if` / `while` branches are now ignored instead of poisoning later ownership state, moving an outer non-`Copy` value inside a `while` body is rejected because the value would not be available on subsequent iterations, post-loop ownership state preserves the pre-loop state since the loop body may execute zero times, and live borrowed slices now block moving their owned collection roots until the borrow scope ends, including when those borrows are wrapped in local tuples, named structs, enum payloads, `Option` / `Result` values, passed through sibling expression evaluation, or introduced by temporary `match` expressions.

This is not the final backend architecture. It is the smallest executable version of the
stage0/stage1 split that can build a native hello-world and carry the 1.0 package model.

## Commands

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/hello --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/hello --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/hello --target "$(rustc -vV | sed -n 's/^host: //p')"
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/hello
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/modules --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace --filter core --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/packages --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/capabilities --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- caps stage1/examples/hello --json
```

`axiomc test` discovers `src/**/*_test.ax` entrypoints by default, builds each test
as a native artifact, executes it, and compares stdout against a sibling
`*.stdout` golden file when present. Projects that need explicit naming or inline
expectations can still declare `[[tests]]` entries in `axiom.toml`. The command
now also accepts `--filter <pattern>` to run a subset of discovered tests by
test name or entry path.

## JSON contract

`axiomc check --json`, `build --json`, `test --json`, and `caps --json` all now
emit the versioned schema envelope `schema_version = "axiom.stage1.v1"`.
Successful payloads always include `ok`, `command`, and `project`, while
`axiomc test --json` additionally reports `filter` and per-run/per-case
`duration_ms`. Build payloads report the requested Rust target triple when
`--target <triple>` is used.

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
- AG1.1 loop-join handling is now landed: moving an outer non-`Copy` value inside a `while` body is a compile error, and post-loop ownership state preserves the pre-loop state since the body may execute zero times. Dead-branch pruning for statically false conditions is preserved.
- Borrowed slices can now flow through direct `&[T]` returns, named structs, enum payloads, and aggregate return types like `Option<&[T]>` or tuples when they are derived from one or more borrowed parameters, `Option` / `Result` match bindings preserve enough borrow provenance to return those borrowed payloads again, conservative call summaries now keep borrowed-return provenance alive across multiple borrowed parameters, statically false control-flow is now skipped instead of contaminating move state, and live borrowed slices now block later owner moves until their scope ends even when those borrows are stored inside local aggregate wrappers, named structs, enum payloads, or temporary `match` / call expressions, but there are still no general borrows, mutable borrows, lifetime checks, precise path-sensitive borrow narrowing beyond constant conditions, or first-class partial-move analysis for aggregates and function calls.
- Exhaustiveness checking now exists for statement-level enum `match`, but there is still no typed error propagation and no control-flow-sensitive ownership diagnostics beyond simple branches.
- Compile-fail coverage now exists for several bootstrap ownership failures including AG1.1 loop-body move rejection, but there is still no dedicated corpus yet for the broader future borrow rules that a Rust-like language actually needs.

### Package and build graph gaps

- `axiom.toml` and `axiom.lock` now support deterministic local path dependency graphs plus package-root workspace members with relative local paths, but there is still no workspace-only manifest or package-selection flow.
- The current import model is still intentionally small: package-local relative path imports plus dependency-prefixed imports like `core/math.ax`, direct `pub struct` / `pub enum` / `pub fn` exports only, and explicit parser diagnostics for unsupported aliases, re-exports, and namespace-qualified calls.
- There is no package registry flow, no version resolution, and no offline lockfile validation beyond the bootstrap lockfile shape.

### Runtime and standard library gaps

- The AG4.1 stdlib surface now covers every stage1 capability-gated intrinsic with a thin wrapper module (`std/time.ax`, `std/env.ax`, `std/fs.ax`, `std/net.ax`, `std/process.ax`, `std/crypto_hash.ax`), plus `std/http.ax` (first stdlib module with a brand-new capability-gated intrinsic `http_get` sharing the existing `net` surface) and `std/io.ax` (first ungated stdlib module, `eprintln` on top of the new `io_eprintln` intrinsic). The remaining AG4.1 modules (`std.json`, `std.collections`, `std.sync`) require new stdlib intrinsics, the AG4.2 async runtime, or AG2 generics and stay as follow-on work.
- Capability enforcement exists for a compiler-known intrinsic slice across all six manifest flags: `fs_read(...)`, `net_resolve(...)`, `process_status(...)`, `env_get(...)`, `clock_now_ms()`, and `crypto_sha256(...)`, and stdlib wrappers preserve that enforcement against the importing package's manifest, but the general stdlib module surface is still mostly empty.
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
- `stage1/examples/stdlib_time` proves the AG4.1 synthetic stdlib surface: `import "std/time.ax"` brings `now_ms()` into scope and remains subject to the importing package's `[capabilities] clock` flag.
- `stage1/examples/stdlib_env` extends AG4.1 with `import "std/env.ax"`, bringing `get_env(key)` into scope and staying subject to the importing package's `[capabilities] env` flag.
- `stage1/examples/stdlib_fs` extends AG4.1 with `import "std/fs.ax"`, bringing `read_file(path)` into scope and staying subject to the importing package's `[capabilities] fs` flag.
- `stage1/examples/stdlib_net` extends AG4.1 with `import "std/net.ax"`, bringing `resolve(host)` into scope and staying subject to the importing package's `[capabilities] net` flag.
- `stage1/examples/stdlib_process` extends AG4.1 with `import "std/process.ax"`, bringing `run_status(command)` into scope and staying subject to the importing package's `[capabilities] process` flag.
- `stage1/examples/stdlib_crypto_hash` extends AG4.1 with `import "std/crypto_hash.ax"`, bringing `sha256(input)` into scope and staying subject to the importing package's `[capabilities] crypto` flag.
- `stage1/examples/stdlib_io` extends AG4.1 with `import "std/io.ax"`, bringing `eprintln(text)` into scope without any capability opt-in — `std/io.ax` is the first stdlib module not tied to a capability flag, matching the ambient status of the `print` statement.
- `stage1/examples/stdlib_http` extends AG4.1 with `import "std/http.ax"`, bringing `get(url)` into scope on top of a new blocking HTTP/1.0 client; it shares the importing package's `[capabilities] net` flag with `std/net.ax` and keeps its smoke deterministic by pointing at a closed local port so the `None` branch always fires.
- `stage1/examples/arrays`, `stage1/examples/maps`, `stage1/examples/tuples`,
  and `stage1/examples/structs` cover the current structured-data floor.
- `stage1/examples/slices`, `stage1/examples/borrowed_shapes`, `stage1/examples/enums`,
  and `stage1/examples/outcomes` cover the current borrow-aware and enum/result floor.
- `make stage1-test stage1-smoke` now covers all twenty checked-in stage1 examples.

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
