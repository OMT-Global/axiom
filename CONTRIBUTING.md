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
```

## Bootstrap discipline
Treat the interpreter and VM as **two implementations** of the same semantics:
- Interpreter (stage0) is the reference.
- VM (stage1) must match the interpreter on conformance tests.
