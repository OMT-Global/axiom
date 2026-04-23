# Python Exit Parity Gate

Status: accepted

Parent issue: [#265](https://github.com/OMT-Global/axiom/issues/265)

Governing issue: [#266](https://github.com/OMT-Global/axiom/issues/266)

This matrix is the Rust-only exit gate for retiring Python `stage0`. A row may
be `ported`, `replaced`, `retired`, or `blocked`. Final Python deletion is
blocked if any row is `blocked`.

## Workflow Matrix

| Python-facing surface | Status | Rust-only path or disposition | Verification |
| --- | --- | --- | --- |
| `python -m axiom check` | replaced | Use `axiomc check <package>` for package and workspace-member checking. | `cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/hello --json` |
| `python -m axiom compile` | replaced | Use `axiomc build <package>` to lower through Rust stage1 and produce generated Rust plus a native binary. | `cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/hello --json` |
| `python -m axiom interp` | retired | There is no supported interpreter mode after Python exit; execute native artifacts with `axiomc run <package>`. | `cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/hello` |
| `python -m axiom vm` | retired | The Python bytecode VM is retired; runtime behavior is owned by Rust crate tests, conformance fixtures, and generated-native execution. | `make stage1-test` and `make stage1-conformance` |
| `python -m axiom repl` | retired | There is no supported REPL in the stage1 workflow. | `bash scripts/ci/check-python-exit-docs.sh` |
| `python -m axiom pkg init` | replaced | Use `axiomc new <path>` to create `axiom.toml`, `axiom.lock`, and starter source. | `cargo run --manifest-path stage1/Cargo.toml -p axiomc -- --help` |
| `python -m axiom pkg build` | replaced | Use `axiomc build <package>`. | `make stage1-smoke` |
| `python -m axiom pkg check` | replaced | Use `axiomc check <package>`. | `make stage1-smoke` |
| `python -m axiom pkg run` | replaced | Use `axiomc run <package>`. | `make stage1-smoke` |
| `python -m axiom pkg clean` | retired | Stage1 build artifacts are ordinary package output under the manifest `out_dir`; remove that directory when needed. | `bash scripts/ci/check-python-exit-docs.sh` |
| `python -m axiom pkg manifest` | retired | `axiom.toml` and `axiom.lock` are the supported metadata surfaces; `axiomc caps <package> --json` reports capability metadata. | `cargo run --manifest-path stage1/Cargo.toml -p axiomc -- caps stage1/examples/hello --json` |
| `python -m axiom host list` | retired | Python host discovery is not part of the Rust-supported execution path. | `bash scripts/ci/check-python-exit-docs.sh` |
| `python -m axiom host describe` | retired | Future host or target inspection should be Rust-owned and tied to native build targets, not Python stage0 hosts. | `bash scripts/ci/check-python-exit-docs.sh` |
| Python language conformance tests | ported | Supported parser, checker, runtime, package, and diagnostic behavior moved into Rust crate tests, `stage1/conformance`, and `axiomc test` fixtures. | `make stage1-test` and `make stage1-conformance` |
| Python package/runtime examples | ported | Checked-in examples under `stage1/examples` are runnable through Rust tooling only. | `make stage1-smoke` |
| Python bytecode format and disassembler | retired | Preserved only as historical material if retained at all; not a compatibility target after Python exit. | `bash scripts/ci/check-python-exit-docs.sh` |

## Blocking Rows

There are no blocked rows in the current matrix.

## Required Gate

The supported Python-exit validation gate is:

```bash
make stage1-test
make stage1-conformance
make stage1-smoke
bash scripts/ci/check-python-exit-docs.sh
bash scripts/check-detect-secrets.sh --all-files
```

`scripts/ci/run-fast-checks.sh` runs the docs check, Rust crate tests,
conformance corpus, and smoke workload for the required PR `CI Gate`.
