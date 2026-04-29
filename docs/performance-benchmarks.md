# Performance Benchmarks

The first benchmark harness is `axiomc bench`. It discovers `*_bench.ax` files,
runs warmup iterations, runs measured iterations, and emits median and p95 wall
time statistics.

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- bench stage1/examples/benchmarks --json
```

The checked-in fixture package lives at `stage1/examples/benchmarks`.

The stage1 baseline harness wraps fixed checked-in examples and records parser,
checker, build, and run timings as JSON:

```bash
make stage1-bench
```

By default the report is written to
`.axiom-build/reports/stage1-bench.json` using schema
`axiom.stage1.bench-harness.v1`. It is a local artifact, not a committed
baseline. Use `scripts/ci/check-stage1-benchmarks.py` for the separate
non-blocking comparison gate against Go/Rust reference workloads.

This closes the local benchmark-suite foundation. Go and Rust reference
comparisons should be layered on top of this harness in CI once representative
workloads are stable enough to treat as performance policy.
