from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

from .errors import Span


class TokenKind(Enum):
    IMPORT = auto()
    LET = auto()
    FN = auto()
    PRINT = auto()
    RETURN = auto()
    IF = auto()
    ELSE = auto()
    AS = auto()
    WHILE = auto()
    FOR = auto()
    IN = auto()
    TRUE = auto()
    FALSE = auto()
    IDENT = auto()
    INT = auto()
    EQ = auto()
    EQEQ = auto()
    NE = auto()
    LT = auto()
    LE = auto()
    GT = auto()
    GE = auto()
    STRING = auto()
    SEMI = auto()
    NEWLINE = auto()
    PLUS = auto()
    MINUS = auto()
    STAR = auto()
    SLASH = auto()
    DOT = auto()
    COLON = auto()
    COMMA = auto()
    LPAREN = auto()
    RPAREN = auto()
    LBRACE = auto()
    RBRACE = auto()
    LBRACKET = auto()
    RBRACKET = auto()
    EOF = auto()


@dataclass(frozen=True)
class Token:
    kind: TokenKind
    span: Span
    value: Optional[object] = None  # e.g., int for INT, str for IDENT
