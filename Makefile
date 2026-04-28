PYTHON ?= python3
AXIOM_BUILD_DIR ?= .axiom-build
ARITH_BYTECODE ?= $(AXIOM_BUILD_DIR)/arith.axb
QUALITY_OUT_DIR ?= .quality

.PHONY: preflight-test-collection test lint smoke interp compile vm stage1-test stage1-smoke stage1-conformance stage1-run coverage coverage-python coverage-rust crap mutation mutation-python mutation-rust

preflight-test-collection:
	bash scripts/ci/preflight-test-collection.sh

test:
	$(PYTHON) -m unittest discover -v

lint:
	$(PYTHON) -m ruff check .

smoke:
	$(PYTHON) -m axiom check examples/arith.ax
	$(PYTHON) -m axiom check tests/programs/bool_values.ax
	$(PYTHON) -m axiom pkg check examples/typed_package
	$(PYTHON) -m axiom pkg run examples/typed_package

interp:
	$(PYTHON) -m axiom interp examples/arith.ax

compile:
	mkdir -p "$(AXIOM_BUILD_DIR)"
	$(PYTHON) -m axiom compile examples/arith.ax -o "$(ARITH_BYTECODE)"

vm: compile
	$(PYTHON) -m axiom vm "$(ARITH_BYTECODE)"

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

coverage: coverage-python coverage-rust

coverage-python:
	PYTHON="$(PYTHON)" QUALITY_OUT_DIR="$(QUALITY_OUT_DIR)" bash scripts/quality/coverage-python.sh

coverage-rust:
	QUALITY_OUT_DIR="$(QUALITY_OUT_DIR)" bash scripts/quality/coverage-rust.sh

crap:
	$(PYTHON) scripts/quality/crap_indicators.py \
		--python-coverage "$(QUALITY_OUT_DIR)/coverage/python.json" \
		--rust-lcov "$(QUALITY_OUT_DIR)/coverage/rust.lcov" \
		--json-out "$(QUALITY_OUT_DIR)/crap.json" \
		--markdown-out "$(QUALITY_OUT_DIR)/crap.md"

mutation: mutation-python mutation-rust

mutation-python:
	PYTHON="$(PYTHON)" bash scripts/quality/mutation-python.sh

mutation-rust:
	bash scripts/quality/mutation-rust.sh
