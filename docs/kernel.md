# Axiom kernel (v0.4)

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
- function calls: `<name>(<arg1>, ... )`
- `host.<name>(...)` for host bridge calls (reserved namespace)

## Expressions
- integer literals: `123`, `-5`
- variables: `x`
- binary ops: `+ - * / == != < <= > >=`
- parentheses: `( ... )`
- unary negation: `-<expr>`
- call expressions: `name(arg1, arg2, ...)`
- host calls: `host.version()`, `host.print(value)`, `host.read(prompt)` (gated)

## Execution
- single file program
- lexical scopes resolve from innermost to outermost
- all variables must be defined before use
- integer division truncates toward zero
- functions use explicit call frames with locals + return address
- every function returns an `int` (implicit `0` if no explicit return is reached)
- `host.print` and `host.read` are side-effecting; they require an explicit runtime flag when enabled
