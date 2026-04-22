# Stage1 Conformance

Run the Rust-owned conformance corpus with:

```sh
make stage1-conformance
```

Packages under `pass/` are executable fixtures. Each package is a complete
stage1 project with `axiom.toml`, `axiom.lock`, source, and
`expected-output.txt`. The conformance runner compiles each discovered
`src/**/*_test.ax` target through the Rust path, executes the generated binary,
and compares stdout to the package-level expected output.

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

Packages under `fail/` are compile-fail fixtures. Each package is a complete
stage1 project with `axiom.toml`, `axiom.lock`, source, and
`expected-error.json`. The conformance runner checks the diagnostic kind, code,
message, relative path, line, and column.
