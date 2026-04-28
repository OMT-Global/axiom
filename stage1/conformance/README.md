# Stage1 Conformance Corpus

This corpus is the Rust-owned replacement target for the Python stage0 golden
programs under `tests/programs/`.

Run the current corpus with:

```bash
make stage1-conformance
```

or directly:

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/conformance --json
```

The initial corpus is intentionally small. It establishes the checked-in
fixture shape and command gate for #267. Additional stage0 behaviors should be
ported here by category until Python no longer owns language conformance.

Pass fixtures live under `pass/` and run as workspace packages with discovered
`src/**/*_test.ax` entrypoints. Compile-fail fixtures live under `fail/`; each
case is a package with `src/main.ax` plus `expected-error.json`, and `axiomc
test` checks the stable diagnostic fields. The initial fail corpus covers type,
import, manifest, dependency, and lockfile diagnostics.
