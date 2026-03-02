from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

from .errors import Span


class TokenKind(Enum):
    LET = auto()
    PRINT = auto()
    IDENT = auto()
    INT = auto()
    EQ = auto()
    SEMI = auto()
    NEWLINE = auto()
    PLUS = auto()
    MINUS = auto()
    STAR = auto()
    SLASH = auto()
    LPAREN = auto()
    RPAREN = auto()
    EOF = auto()


@dataclass(frozen=True)
class Token:
    kind: TokenKind
    span: Span
    value: Optional[object] = None  # e.g., int for INT, str for IDENT
