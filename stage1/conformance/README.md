# Stage1 Conformance

Run the Rust-owned conformance corpus with:

```sh
make stage1-conformance
```

Packages under `fail/` are compile-fail fixtures. Each package is a complete
stage1 project with `axiom.toml`, `axiom.lock`, source, and
`expected-error.json`. The conformance runner checks the diagnostic kind, code,
message, relative path, line, and column.

