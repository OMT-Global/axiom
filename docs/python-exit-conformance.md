# Python Exit Conformance Migration

This document is the migration plan for
[#267](https://github.com/OMT-Global/axiom/issues/267). It defines how the
Python `tests/` coverage moves into Rust-owned stage1 fixtures so `axiom/` can
stop being the language/runtime oracle.

## Current Inventory

The Python test tree currently protects several different surfaces:

| Source | Current coverage | Stage1 destination |
| --- | --- | --- |
| `tests/test_conformance.py` and `tests/programs/*.ax` | 39 golden programs run through interpreter and VM parity | Move supported programs into the stage1 conformance corpus. |
| `tests/test_cli_runtime.py` | `python -m axiom` command behavior, REPL, bytecode JSON, disassembly, host flags | Split into Rust CLI tests for supported `axiomc` workflows; REPL and bytecode cases wait on #247 and #269. |
| `tests/test_cli_packages.py` | Python `axiom.json` package init/build/run/check/manifest/clean behavior | Replace with `axiom.toml`, `axiom.lock`, workspace, package graph, and `axiomc` command tests under #268. |
| `tests/test_errors_core.py` | Parser, checker, host-call, runtime, suggestion, and closure diagnostics | Move supported compile-time cases into a Rust compile-fail corpus; closure-only cases wait on stage1 closure support. |
| `tests/test_errors_imports.py` | Import validation, aliases, reserved namespaces, missing modules, path escapes | Move to stage1 package graph and import fixture tests; keep path escape coverage as a required fixture. |
| `tests/test_errors_runtime.py` | Runtime errors, host calls, bytecode compatibility, array/index behavior, side effects | Move supported runtime behavior into stage1 pass/fail fixtures; bytecode and Python host bridge cases wait on #269/#268. |
| `tests/test_bytecode.py` | Bytecode encoding, decoding, limits, and compatibility | Retire if #269 retires bytecode; port only if bytecode survives as a Rust-owned format. |
| `tests/test_intops.py` | Integer division/modulo semantics | Move into Rust operator tests and at least one conformance pass fixture. |
| `tests/test_loader.py` | Source path loading and traversal rejection | Move to stage1 package/import path safety fixtures. |
| `tests/test_semantic_plan.py` | Python checker planning metadata | Retire unless an equivalent Rust diagnostic/planning contract is exposed. |
| `tests/test_detect_secrets_script.py` | Repository secret-scan script tests | Non-language infrastructure. Keep or port under #270, but do not let it block deleting `axiom/`. |
| `tests/test_quality_crap.py` | Python quality-report helper tests | Quality tooling only. Port or delete when Python tooling leaves the repo. |

## Golden Program Destinations

The stage0 golden programs should be migrated by behavior, not copied blindly
into a legacy compatibility layer.

| Python fixtures | Current behavior | Stage1 action |
| --- | --- | --- |
| `arith`, `div`, `expr_stmt`, `vars`, `let_infer`, `assign`, `assign_outer`, `bool_values`, `if_else`, `while_sum`, `for_basic`, `for_nested`, `scopes` | Core expressions, control flow, binding, assignment, and scoping | Port into stage1 pass fixtures. These are baseline language behavior. |
| `string_concat`, `string_escape`, `string_builtins` | String literals and string helper builtins | Port string syntax and supported stdlib wrappers; replace Python `host.string.*` calls with stage1 standard-library equivalents. |
| `array_basic`, `array_empty`, `array_fn`, `array_len`, `array_neg_index`, `array_push`, `array_set`, `array_strings`, `array_while` | Array literals, indexing, length, functional mutation helpers, array use in functions and loops | Port supported collection behavior through stage1 arrays/slices/stdlib collection fixtures. Retire Python `host.array.*` names unless #268 preserves them as wrappers. |
| `fn_basic`, `fn_nested`, `fn_recursive`, `fn_scope` | Function declaration, nested functions, recursion, and function-local scope | Port supported function cases. Nested function behavior must match the stage1 language decision before deletion. |
| `fn_first_class_basic`, `fn_first_class_param`, `fn_first_class_reassign`, `fn_closure_capture`, `fn_closure_recursive`, `fn_closure_shadow` | First-class functions and closure capture semantics | Blocked until stage1 owns closures or explicitly retires Python closure semantics. |
| `fn_host_abs`, `fn_host_math_abs`, `fn_host_version`, `math_builtins` | Python host bridge and math builtins | Replace with `axiomc caps`, manifest capabilities, and stage1 stdlib/intrinsic fixtures. Do not preserve Python host names unless #268 requires compatibility aliases. |

Stage1 already has useful examples under `stage1/examples/` for arrays, slices,
maps, tuples, structs, enums, packages, modules, capabilities, workspaces, and
standard-library packages. Those examples are good smoke coverage, but #267
still needs a dedicated conformance corpus with expected outputs and expected
diagnostics so Python is no longer the reference.

An initial pass corpus now lives under `stage1/conformance/pass/` and is
runnable with `make stage1-conformance`. It covers core language behavior,
collections, module imports, structs, enums, `match`, `Option`, and `Result`.
An initial compile-fail corpus lives under `stage1/conformance/fail/`; `axiomc
test stage1/conformance --json` now checks those fixtures against
`expected-error.json` alongside the pass corpus. The fail corpus currently
covers type mismatches, non-bool conditions, undefined functions, missing
imports, import aliases, private imported functions, missing dependencies,
workspace traversal rejection, and lockfile mismatch. The rest of this document
tracks the remaining migration buckets.

## Target Fixture Layout

#267 should add a checked-in Rust-owned corpus with this shape:

```text
stage1/conformance/
  pass/
    arithmetic/
      axiom.toml
      axiom.lock
      src/main_test.ax
      src/main_test.stdout
  fail/
    import_escape/
      axiom.toml
      axiom.lock
      src/main.ax
      expected-error.json
```

Pass fixtures should build and run through `axiomc test`, then compare stdout to
the sibling `*_test.stdout` golden file. Executable smoke fixtures may also
include `src/main.ax` when the behavior specifically needs `axiomc run`. Fail
fixtures run through `axiomc test` as compile-fail checks and compare stable
diagnostic fields from `expected-error.json`.

The conformance command is Rust-owned and runnable without importing Python:

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/conformance --json
```

If the command spelling changes, #267 must update this document and the issue
acceptance criteria with the exact replacement.

## Required Migration Buckets

#267 should close only after these buckets are represented:

- Core pass fixtures: arithmetic, booleans, strings, variables, assignment,
  conditionals, loops, functions, recursion, arrays/collections, and package
  imports.
- Core fail fixtures: undefined names, invalid assignment, wrong argument
  counts, type mismatches, non-bool conditions, import path escape, missing
  imports, reserved namespaces, and package graph errors.
- Runtime fixtures: supported division/index/collection behavior and supported
  capability or stdlib calls.
- CLI/package fixtures: `axiomc new`, `check`, `build`, `run`, `test`, and
  `caps` coverage that replaces user-facing Python command tests.
- Explicit retirement notes: every Python bytecode, VM, REPL, host bridge, and
  closure case must link to #247, #268, or #269 if it is not ported.

## Acceptance Checklist

- Every Python test module listed in the inventory has a destination or a
  retirement reason.
- Every supported `tests/programs/*.ax` behavior has a stage1 pass fixture.
- Every supported Python diagnostic category has a stage1 fail fixture with
  stable JSON assertions.
- Bytecode, VM, REPL, Python host bridge, and closure-only behavior are either
  ported to Rust or explicitly retired in the linked issue.
- `make stage1-test` and `make stage1-smoke` pass.
- The new conformance command runs without importing `axiom/` or executing
  Python stage0.

## Verification

Current Rust gates:

```bash
make stage1-test
make stage1-smoke
make stage1-conformance
```

Direct #267 conformance gate:

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/conformance --json
```

The final proof bundle for deleting Python in #272 should include the three
commands above plus the Rust-only CI gate from #270.
