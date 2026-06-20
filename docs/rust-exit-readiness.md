# Rust Exit Readiness

This matrix defines the technical bar for making Rust and Cargo unnecessary for
the supported Axiom toolchain.

The current supported implementation remains the Rust-hosted `stage1/axiomc`
compiler with generated-Rust backend support. Rust exit is complete only when
the official check, build, run, test, documentation, LSP, and release paths can
operate from AxiOM-owned sources and direct native artifacts without requiring
Cargo, generated Rust, or `rustc`.

Final Rust bootstrap issue: [#721](https://github.com/OMT-Global/axiom/issues/721)

## Readiness Command

`make rust-exit-readiness` runs the non-blocking local readiness check:

```bash
make rust-exit-readiness
```

It emits `axiom.rust_exit.readiness.v1` JSON and fails while live blocker issues
remain listed in `docs/rust-exit-readiness.json`, while listed blockers are
closed or unavailable, or while the
machine-readable direct-native runtime ABI reports `ready: false`. Deletion or
release-chain PRs can require live GitHub state:

```bash
make rust-exit-readiness-github
```

The readiness gate is an evidence surface, not permission to remove Rust by
itself. It uses the manifest, the direct-native ABI contract, self-hosted
command/MIR boundary fixtures, and live issue state; this Markdown page is
descriptive evidence only. Closing #721 also requires the governing issues and
review gates to be satisfied.

## Backend Matrix

| Surface | Required state | Current disposition | Governing issue |
| --- | --- | --- | --- |
| Direct native parity matrix | Every supported stage1 surface has a direct-native status row and linked blocker when incomplete. | Implemented as the checked runtime ABI matrix; individual incomplete rows still block through #1124. | [#1124](https://github.com/OMT-Global/axiom/issues/1124) |
| Direct native runtime ABI | Supported values, ownership shapes, stdlib calls, and capability host calls lower through backend-neutral direct-native runtime entrypoints. | Implemented and checked by `scripts/ci/check-direct-native-runtime-abi.py`. | contract |
| Direct native diagnostics and evidence | Direct native builds preserve source diagnostics, provenance, debug manifests, and operator evidence without generated Rust. | Implemented for the Cranelift direct-native spike; broader coverage remains gated by runtime ABI readiness. | [#1124](https://github.com/OMT-Global/axiom/issues/1124) |
| Default backend | `axiomc build` defaults to direct native output and no longer invokes `rustc` for supported broad builds. | Historical default-backend blockers are closed; final release-chain removal is now governed by #721. | [#721](https://github.com/OMT-Global/axiom/issues/721) |
| Generated-Rust removal | The generated-Rust backend and `--backend rust` compatibility path are removed after a release with direct native as default. | Historical generated-Rust removal blockers are closed; final supported-toolchain removal is now governed by #721. | [#721](https://github.com/OMT-Global/axiom/issues/721) |

## Bootstrap Matrix

| Surface | Required state | Current disposition | Governing issue |
| --- | --- | --- | --- |
| AxiOM compiler source layout | Parser, checker, lowering, MIR, backend selection, diagnostics, packages, manifests, lockfiles, and command dispatch have AxiOM package boundaries. | Implemented as [AxiOM Compiler Source Layout and Self-Hosting Boundary](axiom-compiler-source-layout.md); final Rust-bootstrap removal remains governed by #721. | [#721](https://github.com/OMT-Global/axiom/issues/721) |
| Snapshot bootstrap | A previously shipped `axiomc` snapshot builds the next working `axiomc` binary without invoking Cargo. | Final release-chain bootstrap removal remains governed by #721. | [#721](https://github.com/OMT-Global/axiom/issues/721) |
| Final readiness gate | The Rust-exit command proves supported workflows, release builds, tests, docs, and LSP no longer require Rust-only infrastructure. | Implemented as `make rust-exit-readiness`; the gate still fails until the live blocker manifest is empty and evidence checks pass. | [#721](https://github.com/OMT-Global/axiom/issues/721) |
| Compiler verification | Compiler-internal coverage is expressed in AxiOM property form instead of Rust-only tests. | Historical verification blockers are closed; final release-chain proof is governed by #721. | [#721](https://github.com/OMT-Global/axiom/issues/721) |
| Documentation generator | `axiomc doc` and structured/Markdown output are produced by AxiOM-owned code. | Historical documentation generator blockers are closed; final release-chain proof is governed by #721. | [#721](https://github.com/OMT-Global/axiom/issues/721) |
| LSP server | `axiomc lsp` runs an AxiOM-owned LSP server and protocol stack. | `blocked` | [#731](https://github.com/OMT-Global/axiom/issues/731) |

## Closure Rules

- The direct native backend may replace generated Rust only after the backend
  matrix has no incomplete rows (`partial` or `blocked`).
- A direct-native runtime ABI row may be marked `implemented` only when it has
  runtime-entrypoint or backend-emitted codegen evidence; compiler-side
  Cranelift spike evaluation alone is not sufficient.
- `docs/rust-exit-readiness.json` is the live open-blocker manifest. Closed
  issues must be removed or replaced when `ready` is still false.
- #721 may close only after the live blocker manifest is empty, the backend
  matrix has no incomplete rows, and bootstrap/release boundary checks pass.
- Generated Rust may remain as a compatibility backend until #721 can prove it
  is no longer part of the supported release-chain path.
- Cargo may remain as a developer convenience while #721 is being proven, but it
  may not be required by the official release-chain path.
- Any new blocked row must name an open GitHub issue in
  `docs/rust-exit-readiness.json`; stale closed blockers fail the readiness
  check instead of silently preserving a false sense of progress.

## Rust Capture Check

This gate tracks implementation dependencies only. It does not define Axiom
semantics in Rust terms. Direct native, generated Rust, Cargo, and snapshot
bootstrap details are backend or release-chain implementation concerns.
