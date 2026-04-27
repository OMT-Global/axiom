# Axiom style guide

This document defines the canonical source style for `.ax` files until a native
formatter ships. Treat it as the formatting target for examples, tests,
RFC snippets, and compiler-generated sample code.

The goal matches the spirit of `d2 fmt`: pick one readable layout, apply it
consistently, and avoid personal formatting dialects.

## Core rules

- Use spaces, never tabs.
- Indent block contents by two spaces.
- Keep keywords lowercase (`fn`, `let`, `match`, `return`, `pub`).
- Put a single space after commas and around infix operators.
- Put the opening `{` on the same line as `fn`, `if`, `while`, `match`,
  `struct`, and `enum` headers.
- Prefer one statement per line.
- End files with a trailing newline.

## Imports

- Group imports at the top of the file.
- Keep one `import` per line.
- Sort imports lexicographically within a group.
- Leave one blank line between the import block and the first item or statement.

```axiom
import "core/banner.ax"
import "core/math.ax"

print banner("hello", label())
```

## Functions and control flow

- Use concise names that describe the value or operation.
- Add explicit type annotations where the language requires them; do not add
  redundant commentary around obvious types.
- Keep short function signatures on one line when they fit.
- Break after the header only when the signature becomes hard to scan.
- Indent statements inside `if`, `else`, `while`, and `match` arms by two
  spaces.

```axiom
fn banner(name: string): string {
  return "hello " + name
}

if ready {
  print banner("axiom")
} else {
  print "not ready"
}

match result {
  Some(value) {
    print value
  }
  None {
    print "missing"
  }
}
```

## Data declarations

- Keep struct and enum fields one per line.
- Prefer trailing comments on their own line above the declaration instead of at
  the end of a field line.
- Use compact inline literals only when the whole value is still easy to read.
- Expand literals across lines when they grow beyond a short handful of fields
  or arguments.

```axiom
struct Pipeline {
  name: string
  steps: int
  ready: bool
}

let pipeline: Pipeline = Pipeline {
  name: "stage1",
  steps: 3,
  ready: true,
}
```

## Comments

- Use `#` comments sparingly for intent, invariants, or temporary limitations.
- Prefer explaining why a constraint exists instead of narrating the code.
- Keep comments updated when behavior changes.

```axiom
# Stage1 still requires an explicit fallback arm here.
match maybe_name {
  Some(value) {
    print value
  }
  None {
    print "guest"
  }
}
```

## Tests, docs, and generated snippets

- Apply this style to checked-in examples, conformance fixtures, RFC snippets,
  and README/docs code blocks.
- When existing fixtures use older formatting, clean them opportunistically in
  the same change only if the diff stays easy to review.
- Do not mix formatting-only churn into unrelated feature work.

## Formatter outlook

A future Axiom formatter should preserve this guide's layout defaults unless the
project explicitly revises the guide. If formatter behavior and this document
diverge, update them together in the same pull request.
