.PHONY: test smoke stage1-test stage1-conformance stage1-smoke stage1-run

test: stage1-test

smoke: stage1-smoke

stage1-test:
	cargo test --manifest-path stage1/Cargo.toml

stage1-conformance:
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/conformance --json

stage1-smoke:
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/hello --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/hello --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/hello
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/modules --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/modules --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/modules
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/modules --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/packages --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/packages --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/packages
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/packages --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/workspace --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/workspace --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/workspace
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/workspace_only --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/workspace_only --package workspace-app --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/workspace_only --package workspace-app
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/workspace_only --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/capabilities --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/capabilities --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/capabilities
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/capabilities --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/arrays --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/arrays --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/arrays
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/slices --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/slices --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/slices
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/borrowed_shapes --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/borrowed_shapes --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/borrowed_shapes
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/tuples --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/tuples --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/tuples
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/maps --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/maps --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/maps
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/structs --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/structs --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/structs
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/enums --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/enums --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/enums
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/outcomes --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/outcomes --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/outcomes
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/generic_aggregates --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/generic_aggregates --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/generic_aggregates
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_time --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_time --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_time
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_env --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_env --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_env
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_fs --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_fs --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_fs
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_net --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_net --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_net
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_process --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_process --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_process
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_crypto_hash --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_crypto_hash --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_crypto_hash
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_io --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_io --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_io
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_json --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_json --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_collections --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_collections --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_collections
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/stdlib_collections --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_sync --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_sync --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_sync
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/stdlib_sync --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_async --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_async --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_async
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/stdlib_async --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/stdlib_http --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/stdlib_http --json
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/stdlib_http
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- caps stage1/examples/hello --json

stage1-run:
	cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/hello
