# Stage1 benchmark reference: capabilities

This I/O-oriented reference workload mirrors `stage1/examples/capabilities` closely
enough for the stage1 build regression gate. It exercises filesystem reads,
environment lookup, time, hashing-shaped work, and localhost resolution in the
native Go and Rust baselines.
