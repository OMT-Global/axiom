# Python Exit VM Disposition

Status: accepted

Parent issue: [#265](https://github.com/OMT-Global/axiom/issues/265)

Governing issue: [#269](https://github.com/OMT-Global/axiom/issues/269)

## Decision

Axiom will retire the Python `stage0` interpreter, bytecode compiler, bytecode
format, bytecode VM, and disassembler as supported implementation surfaces.
The Rust `stage1` `axiomc` workflow is the only supported execution path.

The supported path is:

1. Parse, check, lower, and build through `stage1/` Rust code.
2. Execute through generated native artifacts produced by `axiomc`.
3. Prove language and runtime behavior with Rust-owned crate tests,
   `stage1/conformance`, and `axiomc test` package fixtures.

There will be no Rust port of the Python bytecode interpreter or VM as part of
the Python exit.

## Component Disposition

| Component | Disposition |
| --- | --- |
| Python interpreter | Retire. It is not a supported execution mode after Python exit. |
| Python bytecode compiler | Retire. Rust `axiomc` owns lowering and generated-native builds. |
| Python bytecode format | Preserve only as historical material if retained at all. It is not a compatibility target. |
| Python bytecode VM | Retire. Runtime behavior must be owned by Rust `stage1` tests or future Rust runtime code. |
| Python disassembler | Retire with the bytecode VM. Future inspection tools should target Rust-owned IR, generated Rust, debug maps, or a future direct backend. |

## Consequences

- Final Python deletion is not blocked on bytecode VM ownership.
- Any behavior formerly protected by Python VM tests must move into Rust-owned
  coverage before deletion, tracked by
  [#267](https://github.com/OMT-Global/axiom/issues/267).
- CLI, package, and user-facing workflows must stay `axiomc`-owned, tracked by
  [#268](https://github.com/OMT-Global/axiom/issues/268).
- Rust-only CI gates replace dual Python/Rust language gates, tracked by
  [#270](https://github.com/OMT-Global/axiom/issues/270).
- User-facing docs and install paths must not direct users to Python `stage0`,
  tracked by [#271](https://github.com/OMT-Global/axiom/issues/271).
- Source deletion remains the final cleanup, tracked by
  [#272](https://github.com/OMT-Global/axiom/issues/272).
- A future direct native backend remains separate longer-term work and is not
  required for Python exit; see
  [#105](https://github.com/OMT-Global/axiom/issues/105).

## Validation Rule

Docs may describe the Python interpreter and VM only as retired, historical, or
to-be-deleted surfaces. They must not present the Python interpreter, bytecode
VM, bytecode format, or disassembler as supported user-facing execution paths.
