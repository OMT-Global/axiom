from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from typing import List, Optional, Union

from .errors import Span


@dataclass(frozen=True)
class Program:
    stmts: List["Stmt"]


# TypeName is any valid Axiom type string: "int", "string", "bool",
# "int[]", "string[]", "bool[]", or "fn(T1,...):R" for function types.
TypeName = str

_SIMPLE_TYPES = frozenset(["int", "string", "bool"])
_ARRAY_TYPES = frozenset(["int[]", "string[]", "bool[]"])


def element_type(array_type: TypeName) -> TypeName:
    """Return the element type for an array type (e.g. 'int[]' -> 'int')."""
    if array_type.endswith("[]"):
        return array_type[:-2]
    raise AssertionError(f"not an array type: {array_type}")


def make_fn_type(param_types: List[TypeName], return_type: TypeName) -> TypeName:
    """Build a canonical function type string, e.g. 'fn(int,string):bool'."""
    return "fn(" + ",".join(param_types) + "):" + return_type


def parse_fn_type(type_name: TypeName) -> tuple[List[TypeName], TypeName]:
    """Parse 'fn(T1,...):R' -> (param_types, return_type).

    Handles nested fn types by tracking parenthesis depth.
    """
    assert type_name.startswith("fn("), f"not a fn type: {type_name!r}"
    rest = type_name[3:]  # after "fn("
    # Find matching closing paren
    depth = 1
    i = 0
    while i < len(rest) and depth > 0:
        if rest[i] == "(":
            depth += 1
        elif rest[i] == ")":
            depth -= 1
        i += 1
    params_str = rest[: i - 1]
    return_str = rest[i + 1 :]  # after "):"
    if not params_str:
        param_types: List[TypeName] = []
    else:
        # Split by comma, respecting nested parens
        param_types = []
        depth = 0
        start = 0
        for j, ch in enumerate(params_str):
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
            elif ch == "," and depth == 0:
                param_types.append(params_str[start:j].strip())
                start = j + 1
        param_types.append(params_str[start:].strip())
    return param_types, return_str


@dataclass(frozen=True)
class TypeRef:
    name: TypeName
    span: Span


@dataclass(frozen=True)
class Param:
    name: str
    span: Span
    type_ref: Optional[TypeRef] = None


# Statements
@dataclass(frozen=True)
class FunctionDefStmt:
    name: str
    params: List[Param]
    body: "BlockStmt"
    span: Span
    return_type: Optional[TypeRef] = None


@dataclass(frozen=True)
class ReturnStmt:
    expr: "Expr"
    span: Span


@dataclass(frozen=True)
class LetStmt:
    name: str
    expr: "Expr"
    span: Span
    type_ref: Optional[TypeRef] = None


@dataclass(frozen=True)
class AssignStmt:
    name: str
    expr: "Expr"
    span: Span


@dataclass(frozen=True)
class PrintStmt:
    expr: "Expr"
    span: Span


@dataclass(frozen=True)
class ExprStmt:
    expr: "Expr"
    span: Span


@dataclass(frozen=True)
class ImportStmt:
    path: str
    alias: str
    span: Span


@dataclass(frozen=True)
class BlockStmt:
    stmts: List["Stmt"]
    span: Span


@dataclass(frozen=True)
class IfStmt:
    cond: "Expr"
    then_block: BlockStmt
    else_block: Optional[BlockStmt]
    span: Span


@dataclass(frozen=True)
class WhileStmt:
    cond: "Expr"
    body: BlockStmt
    span: Span


@dataclass(frozen=True)
class ForStmt:
    """for <var> in <iterable> { <body> }"""

    var: str
    iterable: "Expr"
    body: BlockStmt
    span: Span


Stmt = Union[
    LetStmt,
    AssignStmt,
    ImportStmt,
    PrintStmt,
    ReturnStmt,
    ExprStmt,
    BlockStmt,
    IfStmt,
    WhileStmt,
    ForStmt,
    FunctionDefStmt,
]


# Expressions
@dataclass(frozen=True)
class IntLit:
    value: int
    span: Span


@dataclass(frozen=True)
class StringLit:
    value: str
    span: Span


@dataclass(frozen=True)
class BoolLit:
    value: bool
    span: Span


@dataclass(frozen=True)
class VarRef:
    name: str
    span: Span


@dataclass(frozen=True)
class UnaryNeg:
    expr: "Expr"
    span: Span


@dataclass(frozen=True)
class CallExpr:
    callee: str
    args: List["Expr"]
    span: Span


class BinOp(Enum):
    ADD = auto()
    SUB = auto()
    MUL = auto()
    DIV = auto()
    EQ = auto()
    NE = auto()
    LT = auto()
    LE = auto()
    GT = auto()
    GE = auto()


@dataclass(frozen=True)
class Binary:
    op: BinOp
    lhs: "Expr"
    rhs: "Expr"
    span: Span


@dataclass(frozen=True)
class ArrayLit:
    elements: List["Expr"]
    span: Span


@dataclass(frozen=True)
class IndexExpr:
    array: "Expr"
    index: "Expr"
    span: Span


Expr = Union[IntLit, StringLit, BoolLit, VarRef, UnaryNeg, Binary, CallExpr, ArrayLit, IndexExpr]


def expr_span(e: Expr) -> Span:
    return e.span
