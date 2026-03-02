# Axiom grammar (v0.2)

Whitespace is generally ignored except newlines, which can terminate statements.

```ebnf
program        := stmt* EOF ;

stmt           := let_stmt
               | assign_stmt
               | print_stmt
               | if_stmt
               | while_stmt
               | block
               | expr_stmt ;

let_stmt       := "let" IDENT "=" expr terminator ;
assign_stmt    := IDENT "=" expr terminator ;
print_stmt     := "print" expr terminator ;
if_stmt        := "if" expr block ("else" block)? ;
while_stmt     := "while" expr block ;
block          := "{" NEWLINE* stmt* "}" ;
expr_stmt      := expr terminator ;

terminator     := ";" | NEWLINE | EOF ;

expr           := equality ;
equality       := comparison (("==" | "!=") comparison)* ;
comparison     := term (("<" | "<=" | ">" | ">=") term)* ;
term           := factor (("+" | "-") factor)* ;
factor         := unary (("*" | "/") unary)* ;
unary          := "-" unary | primary ;
primary        := INT | IDENT | "(" expr ")" ;
```

Comments start with `#` and run to end-of-line.
