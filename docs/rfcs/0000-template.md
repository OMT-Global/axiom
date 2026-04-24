# RFC 0000: Title

Status: Draft

Governing issue: #0000

## Summary

One paragraph describing the proposed language, runtime, stdlib, or tooling
change.

## Motivation

Describe the user problem, why the current stage1 behavior is insufficient, and
why the change belongs in Axiom's public contract.

## Non-Goals

List closely related work that this RFC intentionally does not solve.

## Design

Specify the exact user-facing behavior. Include syntax, type rules, runtime
semantics, capability requirements, package interactions, and diagnostics as
needed.

## Examples

Show the smallest useful Axiom programs that should compile, run, or fail after
the RFC is implemented.

```axiom
fn main(): int {
  return 0
}
```

## Implementation Plan

Name the code areas that need changes, such as parser, HIR, MIR, checker,
codegen, runtime intrinsics, stdlib modules, package manifest handling, docs,
and CI.

## Validation

List the concrete fixtures, Rust tests, examples, or Make targets that will
prove the behavior. Prefer Rust-owned `stage1/` coverage:

- `make stage1-test`
- `make stage1-conformance`
- `make stage1-smoke`

## Compatibility

Describe whether existing Axiom source keeps working and whether any migration
or diagnostic path is needed.

## Security And Capability Impact

State whether the design changes host access, runtime determinism, capability
enforcement, package trust, or generated-native execution behavior.

## Alternatives

Record the main alternatives considered and why they were rejected.

## Open Questions

List unresolved questions that must close before implementation starts.

