from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

from .errors import Span


class TokenKind(Enum):
    LET = auto()
    FN = auto()
    PRINT = auto()
    RETURN = auto()
    IF = auto()
    ELSE = auto()
    WHILE = auto()
    IDENT = auto()
    INT = auto()
    EQ = auto()
    EQEQ = auto()
    NE = auto()
    LT = auto()
    LE = auto()
    GT = auto()
    GE = auto()
    SEMI = auto()
    NEWLINE = auto()
    PLUS = auto()
    MINUS = auto()
    STAR = auto()
    SLASH = auto()
    COMMA = auto()
    LPAREN = auto()
    RPAREN = auto()
    LBRACE = auto()
    RBRACE = auto()
    EOF = auto()


@dataclass(frozen=True)
class Token:
    kind: TokenKind
    span: Span
    value: Optional[object] = None  # e.g., int for INT, str for IDENT
