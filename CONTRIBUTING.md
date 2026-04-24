# Contributing

This repo is intentionally small and test-driven.

## Rules
- Keep the language kernel small and specified (`docs/kernel.md`).
- Use the RFC process in `docs/rfcs/` for language-level or runtime-level
  contract changes.
- Add features only with:
  - a spec update, and
  - at least one Rust-run conformance or package test under `stage1/`.

## Running tests
```bash
make stage1-test
make stage1-conformance
make stage1-smoke
```

## Bootstrap discipline
Treat the repo as a staged bootstrap:
- Rust `stage1/` is the supported compiler and runtime path.
- Language behavior should be proven with Rust crate tests, `stage1/conformance`,
  and `axiomc test` package fixtures.
