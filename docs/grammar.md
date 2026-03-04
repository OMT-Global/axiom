# Axiom grammar (v0.6)

Whitespace is generally ignored except newlines, which can terminate statements.

```ebnf
program        := stmt* EOF ;

stmt           := let_stmt
               | assign_stmt
               | import_stmt
               | fn_stmt
               | return_stmt
               | print_stmt
               | if_stmt
               | while_stmt
               | block
               | expr_stmt ;

import_stmt    := "import" STRING terminator ;
fn_stmt        := "fn" IDENT "(" params? ")" block ;  # IDENT and params may not be "host"
params         := IDENT ("," IDENT)* ;
return_stmt    := "return" expr terminator ;
let_stmt       := "let" IDENT "=" expr terminator ;
assign_stmt    := IDENT "=" expr terminator ;
print_stmt     := "print" expr terminator ;
if_stmt        := "if" expr block ("else" block)? ;
while_stmt     := "while" expr block ;
block          := "{" NEWLINE* stmt* "}" ;
expr_stmt      := expr terminator ;
call_expr      := IDENT ("." IDENT)* "(" args? ")" ;  # dotted call namespace restricted to host.*
args           := expr ("," expr)* ;

terminator     := ";" | NEWLINE | EOF ;

expr           := equality ;
equality       := comparison (("==" | "!=") comparison)* ;
comparison     := term (("<" | "<=" | ">" | ">=") term)* ;
term           := factor (("+" | "-") factor)* ;
factor         := unary (("*" | "/") unary)* ;
unary          := "-" unary | primary ;
primary        := INT | IDENT | call_expr | "(" expr ")" ;
STRING         := double-quoted UTF-8 string ;
```

Comments start with `#` and run to end-of-line.
