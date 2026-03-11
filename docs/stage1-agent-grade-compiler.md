# Stage1 Agent-Grade Compiler Plan

This doc is the implementation spec for turning `stage1/` into Axiom's first
workable compiler for agent use. `docs/stage1.md` stays as the shorter status
and slice summary; this file is the detailed execution contract for future work.

## Current baseline

AG0 is the current entry floor and must remain intact before any downstream work starts.

- Stage1 already has a real `axiomc` CLI with `new`, `check`, `build`, `run`, and `caps`.
- The backend is still generated Rust plus `rustc`. That is acceptable for the
  agent-grade milestone as long as the public workflow is fully `axiomc`-driven.
- The current language floor includes multi-file modules, structs, enums,
  arrays, maps, tuples, borrowed slices, `Option<T>`, `Result<T, E>`, and the
  ownership/bootstrap work captured by `stage1/examples/borrowed_shapes`.
- The required verification gate remains:
  - `python -m unittest discover -v`
  - `make stage1-test stage1-smoke`

Entry rule:

- Every AG1+ branch must start from a commit that includes the borrowed-projection
  baseline and the `borrowed_shapes` example.

## Definition of agent-grade compiler

The first workable-compiler bar is **agent-grade**, not direct-native parity.

To count as agent-grade:

- Stage1 must provide a complete end-user workflow through `axiomc`, without
  depending on stage0 to build or run stage1 programs.
- The required public commands at this bar are:
  - `axiomc new`
  - `axiomc check`
  - `axiomc build`
  - `axiomc run`
  - `axiomc test`
  - `axiomc caps`
- The generated-Rust backend remains an internal implementation detail and is
  acceptable for this milestone.
- The compiler must support real Axiom packages for three proof workloads:
  - multi-package CLI
  - queue-style worker
  - small HTTP service
- All three workloads must use capability-gated stage1 stdlib/runtime APIs.
- JSON diagnostics are required on `check`, `build`, `test`, and `caps`.

Not required for the agent-grade bar:

- replacing generated Rust with a direct backend
- `fmt`, `bench`, `doc`, `publish`, registry publishing, or LSP
- trait bounds, macros, higher-kinded abstractions, or user `unsafe`

## Milestones

### AG0: Baseline freeze and entry criteria

Status: landed.

Deliverables:

- Borrowed slices remain valid inside named structs and enum payloads.
- `stage1/examples/borrowed_shapes` stays in the checked-in example set.
- `make stage1-test stage1-smoke` covers the current example matrix.
- `docs/stage1.md` remains the short status page and links to this doc.

Acceptance:

- No AG1+ work may remove or weaken the borrowed-projection regressions.
- Stage1 baseline behavior is proven by the existing Rust suite plus both repo-wide gates.

### AG1: Finish ownership and borrowing

Goal: replace the remaining bootstrap ownership special cases with a stable lexical borrow model.

Work packages:

- `AG1.1`: unknown-branch and loop join handling
  - Add conservative merge rules for non-constant `if` / `while` paths.
  - Keep dead-branch pruning for constant false paths.
- `AG1.2`: mutable borrows
  - Start with borrowed locals and borrowed slices.
  - Reject double mutable borrow and mutable-plus-shared aliasing.
- `AG1.3`: projection-sensitive ownership
  - Stop conservatively consuming whole aggregates when a field or payload move can be represented safely.
  - Recheck call lowering, destructuring, and `match` lowering against that model.
- `AG1.4`: diagnostics and failure corpus
  - Add stable ownership error kinds in JSON diagnostics.
  - Lock a compile-fail suite for move-after-use, invalid returned borrows,
    conflicting borrows, and loop/control-flow hazards.

Acceptance:

- Ownership is no longer described as bootstrap-only in docs.
- The Rust regression suite includes a dedicated ownership compile-fail corpus.
- Stage1 has at least one checked-in ownership-heavy example that passes through `axiomc build` and `axiomc run`.

### AG2: Minimum generic abstraction layer

Goal: add the smallest generic system needed for agent/service code.

Work packages:

- `AG2.1`: monomorphized generic functions
  - Support generic utility functions over existing stage1 types.
  - Require explicit type arguments when inference is ambiguous.
- `AG2.2`: generic structs and enums
  - Support generic wrappers over arrays, maps, slices, `Option<T>`, and `Result<T, E>`.
  - Keep codegen monomorphized.
- `AG2.3`: borrow-generic interaction rules
  - Make borrowed data legal inside generic signatures and generic wrappers.
  - Add compile-fail coverage for mismatched instantiations, unconstrained type
    parameters, and borrowed generic return misuse.

Deliberate exclusions:

- no trait bounds
- no methods
- no higher-kinded abstractions
- no macros
- no requirement for user-defined closures at this milestone

Acceptance:

- Stage1 examples can express generic wrappers and utility helpers without stage0 assistance.
- Generic borrow behavior is covered by both positive and compile-fail tests.

### AG3: Package graph, module rules, and capability enforcement

Goal: make stage1 usable across real multi-package codebases.

Work packages:

- `AG3.1`: dependencies and workspaces
  - Accept dependency entries and workspace membership in `axiom.toml`.
  - Validate `axiom.lock` against the resolved graph.
- `AG3.2`: stable module/import rules
  - Lock the import model for package-local modules plus dependency imports.
  - Reject unsupported aliasing/re-export behavior explicitly rather than implicitly.
- `AG3.3`: capability enforcement
  - Move capability handling from metadata-only to compile/build/run enforcement.
  - Capability-denied programs must fail before native execution.

Acceptance:

- `axiomc check/build/run` works on a workspace with at least one dependency edge.
- `axiom.lock` participates in deterministic builds and is validated in CI.
- Capability-denied code fails consistently with machine-readable diagnostics.

### AG4: Service-grade runtime surface

Goal: provide the minimum runtime and stdlib needed for agents, workers, and small services.

Work packages:

- `AG4.1`: stdlib surface
  - `std.io`
  - `std.fs`
  - `std.env`
  - `std.time`
  - `std.json`
  - `std.http`
  - `std.process`
  - `std.collections`
  - `std.sync`
  - `std.crypto.hash`
- `AG4.2`: async runtime
  - `async fn`
  - `await`
  - task spawning
  - channels
  - cancellation
  - timeouts
  - `select`
- `AG4.3`: HTTP service support
  - HTTP server support is required at this milestone, not just client support.
- `AG4.4`: capability-aware integration
  - Stdlib operations must be capability-gated instead of acting like implicit host access.

Acceptance:

- Stage1 can build and run a small HTTP service, not just scripts and workers.
- File I/O, JSON, process execution, HTTP client/server, async coordination, and
  cancellation are covered by stage1 integration tests.

### AG5: Agent-grade compiler closure

Goal: make the stage1 public workflow complete enough to call the compiler workable.

Work packages:

- `AG5.1`: `axiomc test`
  - Add a public stage1 test command for package/workspace-level test execution.
- `AG5.2`: stable JSON contract
  - Lock JSON diagnostics for `check`, `build`, `test`, and `caps`.
- `AG5.3`: proof workload fixtures
  - Add checked-in end-to-end examples for:
    - multi-package CLI
    - queue-style worker
    - small HTTP service
- `AG5.4`: CI closure
  - Treat the three proof workloads as blocking acceptance tests in CI.

Agent-grade closure bar:

- A multi-package CLI builds and runs under `axiomc`.
- A queue-style worker builds and runs under `axiomc`.
- A small HTTP service builds and runs under `axiomc`.
- All three use stage1 capability-gated APIs.
- Stage0 is not part of the user-facing workflow for those stage1 programs.

## Public interfaces and contracts

- Manifest contract remains `axiom.toml` plus `axiom.lock`.
- The agent-grade milestone does not promise a direct native backend.
- `axiomc test` is part of the required public surface before AG5 closes.
- JSON diagnostics on `check`, `build`, `test`, and `caps` are part of the public contract at AG5.

## Working rules for agents

- One AG work package per PR. Do not combine ownership, generics, package-graph,
  runtime, and backend work in the same change.
- AG2 work starts only after AG1 ownership behavior is stable enough to represent
  borrowed data inside generic signatures without new bootstrap exceptions.
- AG4 work depends on AG3 capability enforcement. Do not ship stdlib modules
  that bypass capability checks.
- AG5 closure work depends on AG3 and AG4 being functional enough to support the
  CLI, worker, and HTTP-service fixtures.
- Unless a change is truly stage1-only, keep the dual verification gate green:
  - `python -m unittest discover -v`
  - `make stage1-test stage1-smoke`

## Post-threshold follow-ons

After AG5 closes, the next compiler track is:

- replace generated-Rust codegen with a direct native backend
- add `fmt`, `bench`, `doc`, `publish`, and LSP
- add benchmark gates against simple Go and Rust references
