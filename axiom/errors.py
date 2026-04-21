from __future__ import annotations

from dataclasses import dataclass
import re
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


_SECRET_REDACTIONS: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile("gh" + r"p_[A-Za-z0-9]{36}"), "[REDACTED_SECRET]"),
    (re.compile("github" + r"_pat_[A-Za-z0-9_]+"), "[REDACTED_SECRET]"),
    (re.compile("sk" + r"-live-[A-Za-z0-9_-]+"), "[REDACTED_SECRET]"),
    (re.compile("sk" + r"-proj-[A-Za-z0-9_-]+"), "[REDACTED_SECRET]"),
    (re.compile("AK" + r"IA[0-9A-Z]{16}"), "[REDACTED_SECRET]"),
    (
        re.compile("BEGIN " + r"(RSA|OPENSSH|EC) PRIVATE KEY"),
        "BEGIN [REDACTED_SECRET] PRIVATE KEY",
    ),
    (
        re.compile(r"\b(ANTHROPIC_API_KEY|OPENAI_API_KEY|SUDO_PASS|BW_SESSION)=[^\s'\"`]+"),
        r"\1=[REDACTED_SECRET]",
    ),
)


def _sanitize_source_line(line_text: str) -> str:
    sanitized = line_text
    for pattern, replacement in _SECRET_REDACTIONS:
        sanitized = pattern.sub(replacement, sanitized)
    return sanitized


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
            text = _sanitize_source_line(text)
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

    def _location_dict(
        self,
        *,
        span: Optional[Span],
        source: Optional[str],
        path: Optional[str],
    ) -> dict[str, object]:
        payload: dict[str, object] = {
            "path": path,
            "span": (
                {
                    "start": span.start,
                    "end": span.end,
                }
                if span is not None
                else None
            ),
        }
        if span is not None and source is not None:
            line, column = _line_col(source, span.start)
            _, line_text = _line_text(source, span.start)
            payload["line"] = line
            payload["column"] = column
            payload["line_text"] = _sanitize_source_line(line_text)
        return payload

    def to_dict(self) -> dict[str, object]:
        return {
            "kind": self.__class__.__name__,
            "message": self.message,
            "rendered": str(self),
            "location": self._location_dict(
                span=self.span,
                source=self.source,
                path=self.path,
            ),
            "notes": [
                {
                    "message": note.message,
                    "location": self._location_dict(
                        span=note.span,
                        source=note.source,
                        path=note.path,
                    ),
                }
                for note in self.notes
            ],
        }


class AxiomParseError(AxiomError):
    pass


class AxiomCompileError(AxiomError):
    pass


class AxiomBytecodeError(AxiomError, ValueError):
    pass


class AxiomRuntimeError(AxiomError):
    pass


class MultiAxiomError(Exception):
    """Raised when the checker collects multiple errors in a single pass."""

    def __init__(self, errors: list[AxiomCompileError]) -> None:
        super().__init__(f"{len(errors)} error(s) found")
        self.errors = list(errors)

    def __str__(self) -> str:
        return "\n\n".join(str(e) for e in self.errors)

    def to_dict(self) -> dict[str, object]:
        return {
            "kind": "MultiAxiomError",
            "errors": [e.to_dict() for e in self.errors],
        }
