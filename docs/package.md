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
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- publish stage1/examples/hello --registry-dir ./registry/packages --signing-key dev-key
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- registry-index ./registry/packages --base-url https://packages.example.test --out ./registry/index.json
cargo run --manifest-path stage1/Cargo.toml -p axiomc -- registry-validate ./registry/index.json
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

## Publish Contract

Remote publishing is not implemented in stage1, but manifests can now declare
the package metadata that future registry tooling will inspect:

```toml
[publish]
registry = "https://registry.example.test/index"
checksum = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
include = ["src", "axiom.toml", "axiom.lock"]
```

Package identity still comes from `[package].name` and `[package].version`.
`[publish].registry` is validated as an `https://` or `file:` registry source,
`[publish].checksum` must use `sha256:<64 hex characters>`, and include entries
must be relative paths without parent traversal. These fields define the
manifest contract only; `axiomc` does not publish, upload, or contact a remote
registry.

See [stage1.md](stage1.md) for the current compiler, package, and capability
contract.

## Publish and Static Registry Groundwork

`axiomc publish` packs a checked stage1 package into a deterministic `package.axp`, writes an `axiom-signature-v1` sidecar, and copies `axiom.toml` plus `axiom.lock` into a local registry tree at `<packages>/<name>/<version>/`. The command validates the lockfile first and refuses to replace an existing release unless `--allow-overwrite` is passed.

`axiomc registry-index` builds a static JSON index from package release folders laid out as
`<packages>/<name>/<version>/axiom.toml`. Each release may include:

- `package.axp` plus `package.axp.sig` for signed package artifacts
- `axiom-registry.toml` with `yanked = true` and optional `yank_reason`

The generated index records per-release capability manifests, archive/signature URLs,
and yanked status so a simple static host can serve lockfile-friendly package metadata. This is publish and registry-index groundwork for a future hosted registry service, not the hosted service itself.
