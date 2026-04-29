# Axiom package manifest

Stage1 packages use `axiom.toml` with a deterministic `axiom.lock` lockfile.
The `axiom.pkg` manifest format is no longer supported.

## Common Commands

```bash
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- check stage1/examples/hello --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- build stage1/examples/hello --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- run stage1/examples/hello
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- test stage1/examples/modules --json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- caps stage1/examples/hello --json
```

## Manifest Shape

The current stage1 examples document the supported manifest surface:

- `stage1/examples/hello`: single-package baseline.
- `stage1/examples/modules`: package-local modules and discovered tests.
- `stage1/examples/packages`: local path dependencies.
- `stage1/examples/workspace`: package-root workspace members.
- `stage1/examples/workspace_only`: workspace-only roots with
  `--package` selection.
- `stage1/examples/capabilities`: manifest-gated runtime capabilities.

Local path dependencies may declare a bounded version constraint:

```toml
[dependencies]
core = { path = "deps/core", version = "^0.1.0" }
```

Stage1 currently accepts `*`, exact `MAJOR.MINOR.PATCH`, and caret
`^MAJOR.MINOR.PATCH` constraints. The compiler validates the constraint against
the dependency package's `[package].version` while loading the local package
graph and fails deterministically when the versions are incompatible.

See [stage1.md](stage1.md) for the current compiler, package, and capability
contract.
