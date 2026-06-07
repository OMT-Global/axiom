# Compiler Package Graph Boundary

`compiler.package_graph` is the AxiOM-owned package identity and dependency
surface for the self-hosted compiler path. It resolves packages from
`axiom.toml`, `axiom.lock`, source files, and future registry metadata. Cargo is
developer scaffolding for the current Rust-hosted compiler only; Cargo metadata
is not package truth.

## Contract

The package graph accepts these inputs:

- Root package directory.
- Root `axiom.toml`.
- Root `axiom.lock`.
- Local workspace member manifests.
- Local path dependency manifests.
- Package source files reachable from each manifest entrypoint and imports.
- Future registry index records and archive integrity metadata.

It produces a stable `axiom.compiler.package_graph.v1` envelope with:

- The root path, manifest path, and lockfile path.
- One package node for every locked root, workspace, and local dependency
  package.
- Package identity from `[package].name`, `[package].version`, and the locked
  source string.
- Workspace membership and local dependency edges from `axiom.toml`.
- Build entrypoint and output directory from `[build]`.
- Lockfile integrity data matching the checked-in `axiom.lock`.
- Hash inputs needed by build caches, release evidence, and snapshot bootstrap.

The graph must not read `Cargo.toml`, `Cargo.lock`, `cargo metadata`, or
`stage1/Cargo.lock` to decide AxiOM package identity. The Rust implementation may
call Rust code to compute the graph during the bootstrap period, but the
observable contract must stay expressible in AxiOM package terms.

## Package Nodes

Each package node has:

- `name`: the manifest package name.
- `version`: the manifest package version.
- `source`: the lockfile source, such as `path` or `path:members/core`.
- `root`: the package root relative to the repository root.
- `manifest`: the package manifest relative to the repository root.
- `lockfile`: the lockfile used for that package graph evaluation.
- `entry`: the manifest build entrypoint relative to the package root.
- `out_dir`: the manifest build output directory relative to the package root.
- `workspace_members`: local workspace members declared by that package.
- `local_dependencies`: dependency name/path edges declared by that package.

The root package appears first. Remaining package nodes are sorted by locked
source and then package name so independent implementations can compare graphs
deterministically.

## Lockfile Integrity

`axiom.lock` remains the package graph integrity source. Official builds must
reject missing, malformed, or stale lockfiles in locked/offline modes before
source lowering or backend selection. A graph fixture is valid only when its
package identities exactly match the decoded lockfile packages.

The current Rust-hosted compiler already hashes the lockfile into build cache
keys. The self-hosted graph must preserve that behavior: cache keys and release
evidence are invalid unless they bind the manifest hash, lockfile hash, and
source hashes for the same package graph.

## Release-Chain Boundary

For #931, a previously released `axiomc` snapshot must be able to read this
contract and build the next compiler without invoking Cargo. Until that release
chain exists:

- Cargo may run local developer commands that host the stage1 compiler.
- Cargo-vet and `stage1/Cargo.lock` may remain part of the temporary
  Rust-hosted supply-chain gate.
- Official package identity must still come from `axiom.toml` and `axiom.lock`.

The release-chain evidence for package loading must include:

- The package graph fixture validated by
  `make stage1-package-graph-boundary`.
- A successful lockfile validation path for root, workspace, and local
  dependency packages.
- Build cache evidence that includes the lockfile hash.
- Supply-chain output for any host tooling used by the temporary bootstrap
  compiler.

## Fixture

The checked fixture lives at:

- `stage1/compiler-contracts/schemas/axiom.compiler.package_graph.v1.schema.json`
- `stage1/compiler-contracts/snapshots/package-graph.json`

The local validator is:

```bash
make stage1-package-graph-boundary
```

It validates the schema envelope, compares fixture package identity with
`stage1/examples/workspace/axiom.lock`, checks the fixture against
`stage1/examples/workspace/axiom.toml`, and rejects Cargo-derived fields inside
the graph output.
