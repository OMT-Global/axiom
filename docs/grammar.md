# Axiom grammar (v0)

Whitespace is generally ignored except newlines, which can terminate statements.

```ebnf
program        := stmt* EOF ;

stmt           := let_stmt | print_stmt | expr_stmt ;

let_stmt       := "let" IDENT "=" expr terminator ;
print_stmt     := "print" expr terminator ;
expr_stmt      := expr terminator ;

terminator     := ";" | NEWLINE | EOF ;

expr           := term (("+" | "-") term)* ;
term           := factor (("*" | "/") factor)* ;
factor         := INT
               | IDENT
               | "(" expr ")"
               | "-" factor ;
```

Comments start with `#` and run to end-of-line.
