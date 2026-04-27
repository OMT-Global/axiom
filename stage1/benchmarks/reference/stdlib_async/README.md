# Stage1 benchmark reference: stdlib_async

This concurrency-oriented reference workload mirrors `stage1/examples/stdlib_async`
for the stage1 build regression gate. The Go and Rust versions exercise a small
spawn/join/channel-shaped program so benchmark coverage is not limited to the
basic hello path.
