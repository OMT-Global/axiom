from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass(frozen=True)
class Span:
    start: int
    end: int


class AxiomError(Exception):
    def __init__(self, message: str, span: Optional[Span] = None):
        super().__init__(message)
        self.message = message
        self.span = span

    def __str__(self) -> str:
        if self.span is None:
            return self.message
        return f"{self.message} (span {self.span.start}:{self.span.end})"


class AxiomParseError(AxiomError):
    pass


class AxiomCompileError(AxiomError):
    pass


class AxiomRuntimeError(AxiomError):
    pass
