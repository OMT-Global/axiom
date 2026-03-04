from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass(frozen=True)
class Span:
    start: int
    end: int


def _line_col(source: str, offset: int) -> tuple[int, int]:
    if offset < 0:
        offset = 0
    line = source.count("\n", 0, min(offset, len(source))) + 1
    line_start = source.rfind("\n", 0, min(offset, len(source))) + 1
    return line, offset - line_start + 1


def _line_text(source: str, offset: int) -> tuple[int, str]:
    line, _ = _line_col(source, offset)
    start = source.rfind("\n", 0, offset)
    if start == -1:
        start = 0
    else:
        start += 1
    end = source.find("\n", offset)
    if end == -1:
        end = len(source)
    return line, source[start:end].rstrip("\n")


class AxiomError(Exception):
    def __init__(
        self,
        message: str,
        span: Optional[Span] = None,
        source: Optional[str] = None,
        path: Optional[str] = None,
    ):
        super().__init__(message)
        self.message = message
        self.span = span
        self.source = source
        self.path = path

    def __str__(self) -> str:
        if self.span is None:
            return self.message
        if self.source is not None:
            line, col = _line_col(self.source, self.span.start)
            if self.path is not None:
                location = f"{self.path}:{line}:{col}"
            else:
                location = f"{line}:{col}"
            line_num, text = _line_text(self.source, self.span.start)
            width = max(1, self.span.end - self.span.start)
            pointer = " " * (col - 1) + "^" * width
            return (
                f"{self.message} (at {location})\n"
                f"  {line_num:>4} | {text}\n"
                f"      | {pointer}"
            )
        return f"{self.message} (span {self.span.start}:{self.span.end})"


class AxiomParseError(AxiomError):
    pass


class AxiomCompileError(AxiomError):
    pass


class AxiomRuntimeError(AxiomError):
    pass
