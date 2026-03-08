from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from typing import List, Literal, Optional, Union

from .errors import Span


@dataclass(frozen=True)
class Program:
    stmts: List["Stmt"]


TypeName = Literal["int", "string", "bool"]


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


Expr = Union[IntLit, StringLit, BoolLit, VarRef, UnaryNeg, Binary, CallExpr]


def expr_span(e: Expr) -> Span:
    return e.span
