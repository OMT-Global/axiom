# Python Exit Parity Matrix

This matrix is the gate for [#266](https://github.com/OMT-Global/axiom/issues/266).
It defines what must happen before Python `stage0` can stop being the
conformance oracle.

Status values:

- `ported`: Rust `stage1` already owns an equivalent supported workflow.
- `replaced`: Rust has a different supported workflow and Python behavior should
  migrate to that model.
- `retired`: the Python behavior should be removed rather than ported.
- `blocked`: no deletion until the linked issue closes.

## Command Surface

| Python stage0 surface | Current purpose | Rust stage1 surface | Status | Blocker / decision |
| --- | --- | --- | --- | --- |
| `python -m axiom check <file>` | Parse and type-check one source file | `axiomc check <package-or-workspace> --json` | `replaced` | #268 must document package-root-first usage and whether single-file check remains supported. |
| `python -m axiom interp <file>` | Run source through tree-walking interpreter | none | `blocked` | #269 decides whether interpreter mode is retired or reintroduced in Rust. |
| `python -m axiom compile <file> -o <bytecode>` | Compile to `.axb` bytecode | `axiomc build <package>` emits native binary via generated Rust | `blocked` | #269 decides bytecode fate; #268 documents native build migration. |
| `python -m axiom vm <bytecode>` | Execute `.axb` bytecode | `axiomc run <package>` builds/runs native artifact | `blocked` | #269 decides whether bytecode VM is retired or ported. |
| `python -m axiom run <file>` | Compile in memory and execute on Python VM | `axiomc run <package>` | `replaced` | #268 must move quickstarts/examples to package-root Rust execution. |
| `python -m axiom disasm <bytecode>` | Inspect `.axb` bytecode | none | `blocked` | #269 decides whether disassembly is retired, kept for historical bytecode, or replaced by debug/source-map tooling. |
| `python -m axiom repl` | Interactive stage0 REPL | open issue [#247](https://github.com/OMT-Global/axiom/issues/247) | `blocked` | #247 and #268. |
| `python -m axiom pkg init` | Create `axiom.json` style package scaffold | `axiomc new <path> --name <name>` | `replaced` | #268 must declare manifest migration from Python package format to `axiom.toml`. |
| `python -m axiom pkg build` | Build package bytecode | `axiomc build <package>` | `replaced` | #268. |
| `python -m axiom pkg run` | Run package main source | `axiomc run <package>` | `replaced` | #268. |
| `python -m axiom pkg check` | Check manifest and compile main | `axiomc check <package>` | `replaced` | #268. |
| `python -m axiom pkg manifest` | Print Python package manifest JSON | `axiomc check/build/test --json`, `axiom.toml`, `axiom.lock` | `blocked` | #268 decides whether an `axiomc manifest` command is needed or JSON command output is enough. |
| `python -m axiom pkg clean` | Delete Python package artifacts | none | `blocked` | #268 decides whether `axiomc clean` is needed or artifacts remain build-system managed. |
| `python -m axiom host list` | List Python host bridge capabilities | `axiomc caps [path] --json` | `replaced` | #268 must document `caps` as the Rust capability inspection path. |
| `python -m axiom host describe` | Emit Python host contract for tooling | `axiomc caps [path] --json` plus stage1 JSON contracts | `blocked` | #268 decides whether `caps` fully replaces host contract output. |

## Runtime And Language Surfaces

| Python stage0 surface | Current owner | Rust stage1 state | Status | Blocker / decision |
| --- | --- | --- | --- | --- |
| Parser and grammar acceptance | `axiom/parser.py`, `axiom/lexer.py` | `stage1/crates/axiomc/src/syntax.rs` | `blocked` | #267 must map Python parser tests and golden programs to stage1 fixtures. |
| Static checking | `axiom/checker.py`, `axiom/semantic_plan.py` | `hir.rs`, `project.rs`, diagnostics | `blocked` | #267 and #226 for borrow-check separation follow-on. |
| Interpreter semantics | `axiom/interpreter.py` | no Rust interpreter | `blocked` | #269. |
| Bytecode format | `axiom/bytecode.py`, `docs/bytecode.md` | no stage1 bytecode backend | `blocked` | #269. |
| Python VM runtime | `axiom/vm.py` | native run through generated Rust and `rustc` | `blocked` | #269 decides retirement versus Rust port. |
| Host bridge registry | `axiom/host.py` | manifest capabilities, compiler-known intrinsics, `std/*` wrappers | `replaced` | #268 and #271 must update docs. |
| Package model | `axiom/packaging.py`, `docs/package.md` | `axiom.toml`, `axiom.lock`, workspace/package graph | `replaced` | #268 and #271. |
| Error rendering and notes | `axiom/errors.py`, CLI JSON output | `diagnostics.rs`, JSON contracts | `blocked` | #228 and #267 need enough parity for user-facing errors. |
| REPL state model | `axiom/repl.py` | none | `blocked` | #247. |

## Test And CI Surfaces

| Python stage0 surface | Current purpose | Rust replacement | Status | Blocker / decision |
| --- | --- | --- | --- | --- |
| `python -m unittest discover -v` | Full Python stage0 correctness gate | `make stage1-test`, `make stage1-smoke`, future conformance/proof gate | `blocked` | #267 and #270. |
| `tests/test_conformance.py` and `tests/programs/*.ax` | Interpreter/VM parity and golden programs | stage1 conformance fixture corpus | `blocked` | #267. |
| `tests/test_cli_runtime.py` | Python command and VM behavior | Rust CLI tests and stage1 example smoke | `blocked` | #267 and #268. |
| `tests/test_cli_packages.py` | Python package manifest/build/run behavior | Rust package/workspace tests | `blocked` | #267 and #268. |
| `tests/test_errors_*.py` | Python diagnostics/runtime errors | Rust compile-fail corpus and JSON diagnostics tests | `blocked` | #267 and #228. |
| `tests/test_bytecode.py` | Bytecode validation | none unless bytecode survives | `blocked` | #269. |
| `tests/test_loader.py` | Python import loader safety | stage1 package graph/import tests | `blocked` | #267. |
| `tests/test_intops.py` | Python int operation behavior | Rust runtime/operator tests | `blocked` | #267. |
| `tests/test_detect_secrets_script.py` | Repo secret-scan script coverage | Keep Python test or port script tests if Python is removed entirely | `blocked` | #270 decides whether non-language Python tests may remain. |

## Source Deletion Inventory

These files are in scope for the final deletion issue
[#272](https://github.com/OMT-Global/axiom/issues/272) after the blockers close:

- `axiom/__init__.py`
- `axiom/__main__.py`
- `axiom/api.py`
- `axiom/ast.py`
- `axiom/bytecode.py`
- `axiom/checker.py`
- `axiom/cli.py`
- `axiom/compiler.py`
- `axiom/errors.py`
- `axiom/host.py`
- `axiom/interpreter.py`
- `axiom/intops.py`
- `axiom/lexer.py`
- `axiom/loader.py`
- `axiom/packaging.py`
- `axiom/parser.py`
- `axiom/repl.py`
- `axiom/semantic_plan.py`
- `axiom/suggestions.py`
- `axiom/token.py`
- `axiom/values.py`
- `axiom/vm.py`

The Python test tree must be migrated or retired before deletion:

- `tests/test_bytecode.py`
- `tests/test_cli_packages.py`
- `tests/test_cli_runtime.py`
- `tests/test_conformance.py`
- `tests/test_errors_core.py`
- `tests/test_errors_imports.py`
- `tests/test_errors_runtime.py`
- `tests/test_intops.py`
- `tests/test_loader.py`
- `tests/test_semantic_plan.py`
- `tests/helpers.py`
- `tests/programs/*.ax`
- `tests/programs/*.out`

`tests/test_detect_secrets_script.py` is not language-runtime coverage. It can
remain temporarily only if the repo still uses Python for non-Axiom
infrastructure tests, but it must not block deleting `axiom/`.

The detailed #267 migration inventory and target fixture layout live in
[Python Exit Conformance Migration](python-exit-conformance.md).

## Gate Summary

The parity gate is not closed today. The critical blockers are:

- #267 for Rust-owned conformance.
- #268 for supported user workflows.
- #269 for interpreter/bytecode/VM disposition.
- #270 for CI ownership.
- #271 for docs/install migration.

When those blockers close, #272 can delete Python stage0 with a Rust-only proof
bundle.
