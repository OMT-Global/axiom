# Axiom kernel (v0)

This repo starts with an intentionally tiny kernel so we can bootstrap safely.

## Values
- integers only (Python `int` in the seed implementation)
- semantics intended to match an `i64`-like model later (overflow behavior TBD)

## Statements
- `let <ident> = <expr>` binds a variable in the current (global) environment
- `print <expr>` prints the integer result plus a newline

## Expressions
- integer literals: `123`, `-5`
- variables: `x`
- binary ops: `+ - * /` with standard precedence and left-associativity
- parentheses: `( ... )`
- unary negation: `-<expr>`

## Execution
- single file program
- global scope only
- all variables must be defined before use
