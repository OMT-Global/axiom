# Quality Signals

Axiom has two implementation tracks, so quality checks are split by runtime:

- stage0 Python: `axiom/` with `unittest`, `coverage.py`, and `mutmut`.
- stage1 Rust: `stage1/` with `cargo test`, `cargo-llvm-cov`, and `cargo-mutants`.

Mutation testing is intentionally manual. It is expensive enough that it should not block the required PR gate until the thresholds are calibrated from real runs.

## Setup

Install the Python quality tools into the project virtual environment:

```bash
python3 -m pip install -e '.[quality]'
```

Install the Rust cargo subcommands when you need Rust coverage or mutation testing:

```bash
cargo install cargo-llvm-cov --locked
cargo install cargo-mutants --locked
rustup component add llvm-tools-preview
```

## Coverage

Generate both coverage inputs:

```bash
make coverage
```

This writes:

- `.quality/coverage/python.json`
- `.quality/coverage/rust.lcov`

Run only one side when iterating:

```bash
make coverage-python
make coverage-rust
```

## CRAP Indicators

Generate the combined report:

```bash
make crap
```

The report is written to:

- `.quality/crap.md`
- `.quality/crap.json`

CRAP is computed as:

```text
complexity^2 * (1 - coverage)^3 + complexity
```

Python complexity comes from the Python AST. Rust complexity is a lightweight source-level indicator over `fn` bodies and common branch tokens. Rows without coverage still appear with complexity so missing coverage setup is visible.

To fail a run after thresholds are calibrated:

```bash
python3 scripts/quality/crap_indicators.py --fail-on-crap-over 30
```

## Mutation Testing

Run Python mutation testing. `mutmut` uses the `tool.mutmut` configuration in `pyproject.toml` and executes the existing unittest suite through pytest collection:

```bash
make mutation-python
```

Run Rust mutation testing:

```bash
make mutation-rust
```

Run both:

```bash
make mutation
```

Both mutation commands accept extra arguments by invoking the underlying script directly:

```bash
bash scripts/quality/mutation-python.sh --help
bash scripts/quality/mutation-rust.sh --list
```

Treat surviving mutants as targeted test-work backlog. Start with the highest CRAP rows and mutation survivors that touch parser, checker, compiler, VM, package, and stage1 project/runtime behavior.
