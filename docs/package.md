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

See [stage1.md](stage1.md) for the current compiler, package, and capability
contract.

## Editor Schemas

Checked-in editor and agent metadata lives under `stage1/schemas/`:

- `stage1/schemas/axiom.toml.schema.json` describes the decoded `axiom.toml`
  manifest shape for TOML-aware editors.
- `stage1/schemas/axiom.stage1.v1.schema.json` describes the shared JSON
  envelope emitted by `axiomc check`, `build`, `test`, and `caps` with
  `--json`.

These schemas are intentionally metadata for editor completion, validation, and
agent contract discovery. The compiler remains the source of truth for semantic
checks such as dependency graph validity, capability enforcement, and source
analysis.
