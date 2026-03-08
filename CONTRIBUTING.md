# Contributing

This repo is intentionally small and test-driven.

## Rules
- Keep the language kernel small and specified (`docs/kernel.md`).
- Add features only with:
  - a spec update, and
  - at least one conformance test in `tests/programs/`.

## Running tests
```bash
python -m unittest discover -v
python -m ruff check .
make smoke
make stage1-test
make stage1-smoke
```

## Bootstrap discipline
Treat the repo as a staged bootstrap:
- Python `axiom/` is stage0 and remains the reference parser/checker/interpreter/VM.
- Rust `stage1/` is the native compiler bootstrap and may support only a subset while it grows.
- Overlapping language behavior should continue to be proven against stage0 before features are promoted.
