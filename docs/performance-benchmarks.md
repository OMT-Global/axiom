# Performance Benchmarks

The first benchmark harness is `axiomc bench`. It discovers `*_bench.ax` files,
runs warmup iterations, runs measured iterations, and emits median and p95 wall
time statistics.

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- bench stage1/examples/benchmarks --json
```

The checked-in fixture package lives at `stage1/examples/benchmarks`.

Extended validation now emits a non-blocking Go/Rust/Axiom comparison report for
the representative reference workloads under `stage1/benchmarks/reference`.
That report captures build time, run time, binary size, diagnostics JSON shape,
and capability manifest coverage so performance policy can be calibrated before
the comparison becomes a required PR gate.
