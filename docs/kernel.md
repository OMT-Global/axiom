# Axiom kernel (v0.2)

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

## Expressions
- integer literals: `123`, `-5`
- variables: `x`
- binary ops: `+ - * / == != < <= > >=`
- parentheses: `( ... )`
- unary negation: `-<expr>`

## Execution
- single file program
- lexical scopes resolve from innermost to outermost
- all variables must be defined before use
- integer division truncates toward zero
