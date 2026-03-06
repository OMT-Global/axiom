from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass(frozen=True)
class Span:
    start: int
    end: int


@dataclass(frozen=True)
class DiagnosticNote:
    message: str
    span: Optional[Span] = None
    source: Optional[str] = None
    path: Optional[str] = None


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
        notes: Optional[list[DiagnosticNote]] = None,
    ):
        super().__init__(message)
        self.message = message
        self.span = span
        self.source = source
        self.path = path
        self.notes = list(notes) if notes is not None else []

    def add_note(
        self,
        message: str,
        *,
        span: Optional[Span] = None,
        source: Optional[str] = None,
        path: Optional[str] = None,
    ) -> None:
        self.notes.append(
            DiagnosticNote(message=message, span=span, source=source, path=path)
        )

    @staticmethod
    def _render_block(
        message: str,
        span: Optional[Span],
        source: Optional[str],
        path: Optional[str],
        *,
        prefix: str = "",
    ) -> str:
        label = f"{prefix}{message}"
        if span is None:
            return label
        if source is not None:
            line, col = _line_col(source, span.start)
            if path is not None:
                location = f"{path}:{line}:{col}"
            else:
                location = f"{line}:{col}"
            line_num, text = _line_text(source, span.start)
            width = max(1, span.end - span.start)
            pointer = " " * (col - 1) + "^" * width
            return (
                f"{label} (at {location})\n"
                f"  {line_num:>4} | {text}\n"
                f"      | {pointer}"
            )
        return f"{label} (span {span.start}:{span.end})"

    def __str__(self) -> str:
        blocks = [
            self._render_block(self.message, self.span, self.source, self.path)
        ]
        for note in self.notes:
            blocks.append(
                self._render_block(
                    note.message,
                    note.span,
                    note.source,
                    note.path,
                    prefix="note: ",
                )
            )
        return "\n".join(blocks)


class AxiomParseError(AxiomError):
    pass


class AxiomCompileError(AxiomError):
    pass


class AxiomRuntimeError(AxiomError):
    pass
