# Axiom kernel (v0.6)

## Values
- integers only (Python `int` in the seed implementation)
- conditions use truthiness: `0` is false, non-zero is true
- comparisons produce `0` or `1`

## Statements
- `let <ident> = <expr>` binds in the current lexical scope
- `<ident> = <expr>` assigns to nearest existing lexical binding
- `print <expr>` prints the integer result plus a newline
- `{ ... }` introduces a nested lexical scope
- `if <expr> { ... } else { ... }`
- `while <expr> { ... }`
- `fn <name>(<params>) { ... }`
- `return <expr>`
- `import "<path>"` for file module inclusion (resolved relative to file path; loaded at compile time)
  - Import paths must be relative and must not use parent traversal (`..`).
- function calls: `<name>(<arg1>, ... )`
- identifiers named `host` are reserved for host namespace (`let host`, parameters, and function names are rejected)
- `host.<name>(...)` for host bridge calls (reserved namespace)
- Host calls are resolved from a registry. Add custom capabilities via
  `axiom.host.register_host_builtin(name, arity, side_effecting, handler)` where
  `handler(args: list[int], out: TextIO) -> int`.

## Expressions
- integer literals: `123`, `-5`
- variables: `x`
- binary ops: `+ - * / == != < <= > >=`
- parentheses: `( ... )`
- unary negation: `-<expr>`
- call expressions: `name(arg1, arg2, ...)`
- host calls: `host.version()`, `host.print(value)`, `host.read(prompt)`, `host.abs(value)`, `host.math.abs(value)` (gated for side effects only)

## Execution
- single file program
- lexical scopes resolve from innermost to outermost
- all variables must be defined before use
- integer division truncates toward zero
- functions use explicit call frames with locals + return address
- every function returns an `int` (implicit `0` if no explicit return is reached)
- `host.print` and `host.read` are side-effecting; they require an explicit runtime flag when enabled
- non-side-effecting host calls can be used in deterministic tool pipelines without flags
- `python -m axiom host list --safe-only` enumerates host calls that are safe by default
- `host` call payloads in bytecode are name-based (string table index) starting at
  bytecode `v0.6` to preserve behavior if host registry order changes.
- `python -m axiom host list` enumerates the currently registered host capabilities.
