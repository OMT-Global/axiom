from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from typing import List, Union

from .errors import Span


@dataclass(frozen=True)
class Program:
    stmts: List["Stmt"]


# Statements
@dataclass(frozen=True)
class LetStmt:
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


Stmt = Union[LetStmt, PrintStmt, ExprStmt]


# Expressions
@dataclass(frozen=True)
class IntLit:
    value: int
    span: Span


@dataclass(frozen=True)
class VarRef:
    name: str
    span: Span


@dataclass(frozen=True)
class UnaryNeg:
    expr: "Expr"
    span: Span


class BinOp(Enum):
    ADD = auto()
    SUB = auto()
    MUL = auto()
    DIV = auto()


@dataclass(frozen=True)
class Binary:
    op: BinOp
    lhs: "Expr"
    rhs: "Expr"
    span: Span


Expr = Union[IntLit, VarRef, UnaryNeg, Binary]


def expr_span(e: Expr) -> Span:
    return e.span  # all expr nodes have span
