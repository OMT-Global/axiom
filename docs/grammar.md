# Axiom grammar (v0.11)

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

import_stmt    := "import" STRING terminator ;                      # default module namespace from path, using dots for path separators
               | "import" STRING "as" qualified_ident terminator ;  # explicit alias
qualified_ident := IDENT ("." IDENT)* ;
               # imported modules are function-only and may contain imports plus fn declarations.
fn_stmt        := "fn" IDENT "(" params? ")" ":" type_name block ;  # IDENT and params may not be "host"
params         := param ("," param)* ;
param          := IDENT ":" type_name ;
return_stmt    := "return" expr terminator ;
let_stmt       := "let" IDENT ":" type_name "=" expr terminator ;
assign_stmt    := IDENT "=" expr terminator ;
print_stmt     := "print" expr terminator ;
if_stmt        := "if" expr block ("else" block)? ;
while_stmt     := "while" expr block ;
block          := "{" NEWLINE* stmt* "}" ;
expr_stmt      := expr terminator ;
call_expr      := IDENT ("." IDENT)* "(" args? ")" ;  # dotted call namespace: host.* or imported module.*
                  # each import path may appear at most once per file, and alias names must be unique.
args           := expr ("," expr)* ;

terminator     := ";" | NEWLINE | EOF ;
type_name      := "int" | "string" | "bool"
               | type_name "[]"                                       # array type: int[], string[], bool[]
               | "fn" "(" (type_name ("," type_name)*)? ")" ":" type_name ;  # function type: fn(int,string):bool

expr           := equality ;
equality       := comparison (("==" | "!=") comparison)* ;
comparison     := term (("<" | "<=" | ">" | ">=") term)* ;
term           := factor (("+" | "-") factor)* ;
factor         := postfix (("*" | "/") postfix)* ;
postfix        := primary ("[" expr "]")* ;                           # index expressions: xs[i]
unary          := "-" postfix | primary ;
primary        := INT | STRING | "true" | "false" | IDENT | call_expr | "(" expr ")"
               | "[" (expr ("," expr)* ","?)? "]" ;                  # array literal: [1, 2, 3]
STRING         := double-quoted UTF-8 string ;
```

Comments start with `#` and run to end-of-line.
