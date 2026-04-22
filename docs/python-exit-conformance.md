# Python Exit Conformance

Stage1 now owns a Rust-run conformance corpus at `stage1/conformance`.

Run it with:

```sh
make stage1-conformance
```

## Negative Semantic Coverage

Compile-fail fixtures live under `stage1/conformance/fail`. Each fixture is a
complete package and includes `expected-error.json` so the runner checks the
diagnostic kind, code, exact message, relative source path, line, and column.

Current negative fixtures cover:

- `ownership_use_after_move`: rejects use of a moved non-`Copy` string.
- `mutable_borrow_while_shared_live`: rejects mutable borrowing while a shared
  borrow of the same owner is live.
- `result_ok_without_context`: rejects `Ok(...)` without an expected
  `Result<T, E>` type.
- `stdlib_clock_without_capability`: rejects clock runtime intrinsic use when
  `[capabilities].clock` is disabled.

## Executable Rust Coverage

Executable fixtures live under `stage1/conformance/pass`. Each fixture is a
complete package and includes `expected-output.txt` so the runner compiles and
executes the Rust-generated binary, then checks stdout.

Current executable fixtures cover:

- `functions_across_modules`: function calls and return values imported from a
  sibling module.
- `struct_field_access`: struct construction, field access, and passing a
  struct through a function.
- `outcome_control_flow`: `Option` and `Result` construction plus `match`
  control flow.
- `collection_operations`: standard collection helpers over arrays and
  borrowed slices.
- `package_local_modules`: nested package-local module imports that execute
  successfully.

## Remaining Gaps

The corpus should still grow before Python stage0 retirement. Remaining
negative-semantic gaps include broader mutable/shared aliasing shapes, `Err`
and `?` result propagation failures, non-exhaustive and malformed enum/result
matches, runtime boundary denials for the other capability-gated stdlib
surfaces, and package/module visibility edge cases that are not yet represented
as conformance fixtures. Executable gaps include richer enum payloads,
cross-package dependency execution, mutation-heavy collection workflows, and
stdlib capability success paths.
