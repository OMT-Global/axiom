# Python Exit Plan

This plan defines the path from the current dual-track repository to a Rust-only
Axiom implementation.

## Current State

Axiom is not fully migrated out of Python today.

- `stage0` is the Python implementation in `axiom/`. It still owns the original
  parser/checker/interpreter/bytecode VM/package workflow and its Python tests.
- `stage1` is the Rust compiler/toolchain in `stage1/`. It is the forward path,
  but the docs still define Python stage0 as the conformance oracle for
  overlapping behavior.

The goal is to remove that dependency. Rust should become the sole supported
implementation and developer workflow.

## Exit Principle

Python can be deleted only after Rust owns the behavior and workflows users rely
on. Deleting Python without a replacement would lose the reference tests,
package workflow, interpreter/VM decisions, and documentation clarity.

Generated Rust remains acceptable as an internal backend while exiting Python.
The exit bar is Rust-owned public tooling and Rust-owned conformance, not
immediate direct-native codegen. Direct backend work remains tracked separately.

## Exit Bar

Python stage0 can be removed when all of these are true:

- `axiomc` owns supported check, build, run, test, and package workflows.
- Stage1 has a checked-in conformance corpus that replaces Python as the oracle
  for supported language/runtime behavior.
- The CLI, worker, and HTTP-service proof workloads pass as blocking CI.
- The interpreter and bytecode VM have an explicit disposition: port, retire, or
  preserve as historical/spec material.
- Docs no longer instruct users to run `python -m axiom` for supported usage.
- CI no longer runs Python tests to prove Axiom language/runtime correctness.
- Remaining Python, if any, is unrelated bootstrap/ops tooling and not part of
  the supported Axiom implementation.

## Work Items

- [#265](https://github.com/OMT-Global/axiom/issues/265) tracks the overall
  Python-exit roadmap.
- [#266](https://github.com/OMT-Global/axiom/issues/266) defines the Rust-only
  parity gate and command/workflow matrix. The current matrix lives in
  [Python Exit Parity Matrix](python-exit-parity.md).
- [#267](https://github.com/OMT-Global/axiom/issues/267) migrates Python
  conformance coverage into Rust/stage1 fixtures. The current inventory and
  fixture target live in
  [Python Exit Conformance Migration](python-exit-conformance.md).
- [#268](https://github.com/OMT-Global/axiom/issues/268) migrates user-facing CLI
  and package workflows to `axiomc`.
- [#269](https://github.com/OMT-Global/axiom/issues/269) decides the interpreter
  and bytecode VM disposition.
- [#270](https://github.com/OMT-Global/axiom/issues/270) replaces the dual CI gate
  with Rust-only language/runtime gates.
- [#271](https://github.com/OMT-Global/axiom/issues/271) removes Python from the
  supported docs and install path.
- [#272](https://github.com/OMT-Global/axiom/issues/272) removes stage0 source,
  tests, and packaging after the blockers are closed.

Related Rust-forward prerequisites already on the backlog:

- [#101](https://github.com/OMT-Global/axiom/issues/101): proof workload
  fixtures for CLI, worker, and HTTP service.
- [#102](https://github.com/OMT-Global/axiom/issues/102): proof workloads as
  blocking acceptance tests.
- [#105](https://github.com/OMT-Global/axiom/issues/105): replace generated-Rust
  codegen with a direct native backend. This is not required for Python exit,
  but remains part of the longer-term compiler roadmap.

## Execution Order

1. Complete the parity matrix in #266.
2. Port or retire Python conformance coverage in #267.
3. Make `axiomc` own supported CLI and package workflows in #268.
4. Decide interpreter/bytecode VM fate in #269.
5. Promote Rust-only gates and proof workloads in #270.
6. Update docs and install paths in #271.
7. Delete Python stage0 in #272.

## Agent Rules

- Do not remove Python stage0 until #266 through #271 are closed.
- Do not add new user-facing features to Python stage0 unless they are needed to
  preserve current behavior during migration.
- Prefer new language/runtime work in Rust stage1.
- When touching overlapping behavior, add Rust-owned coverage first and keep
  Python coverage only as temporary migration evidence.
- Keep work one issue per branch unless explicitly told to batch.

## Verification Contract

The final Rust-only state should be provable with:

```bash
make stage1-test
make stage1-smoke
```

plus the Rust conformance/proof workload gate defined by #270.
