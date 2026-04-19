from __future__ import annotations

import sys
from dataclasses import dataclass, field
from typing import TextIO

from .ast import AssignStmt, ExprStmt, FunctionDefStmt, LetStmt, Stmt
from .checker import CheckedProgram, check_program
from .errors import AxiomError, AxiomParseError
from .interpreter import Interpreter
from .loader import parse_program
from .values import render_value


INTRO = "Axiom REPL. Type :quit or :exit to leave."
HELP = "Commands: :help, :quit, :exit"
UNKNOWN_COMMAND = "unknown REPL command"
PRIMARY_PROMPT = "axiom> "
CONTINUATION_PROMPT = "... "


def install_history() -> bool:
    try:
        import readline  # noqa: F401
    except ImportError:
        return False
    return True


@dataclass
class ReplResult:
    source: str
    lines: list[str] = field(default_factory=list)


@dataclass
class ReplSession:
    allow_host_side_effects: bool = False
    chunks: list[str] = field(default_factory=list)
    stmt_count: int = 0
    interpreter: Interpreter = field(default_factory=Interpreter)

    def __post_init__(self) -> None:
        self.interpreter.allow_host_side_effects = self.allow_host_side_effects
        self.interpreter.global_scope = self.interpreter.scopes[0]

    def accept(self, source: str, out: TextIO) -> ReplResult:
        candidate_chunks = [*self.chunks, source]
        program = parse_program("\n".join(candidate_chunks), path="<repl>")
        checked = check_program(
            program,
            allow_host_side_effects=self.allow_host_side_effects,
        )

        new_stmts = program.stmts[self.stmt_count :]
        self.interpreter.load_checked_program(checked)

        result = ReplResult(source=source)
        for stmt in new_stmts:
            result.lines.extend(self._execute_repl_stmt(stmt, checked, out))

        self.chunks = candidate_chunks
        self.stmt_count = len(program.stmts)
        return result

    def _execute_repl_stmt(
        self,
        stmt: Stmt,
        checked: CheckedProgram,
        out: TextIO,
    ) -> list[str]:
        if isinstance(stmt, ExprStmt):
            value = self.interpreter.eval_expr(stmt.expr, out)
            typ = checked.expr_types.get(id(stmt.expr), "value")
            return [f"{render_value(value)} : {typ}"]

        self.interpreter.exec_stmt(stmt, out)

        if isinstance(stmt, LetStmt):
            value = self.interpreter.global_scope.get(stmt.name)
            typ = checked.expr_types.get(
                id(stmt.expr),
                stmt.type_ref.name if stmt.type_ref else "value",
            )
            return [f"{stmt.name} : {typ} = {render_value(value)}"]
        if isinstance(stmt, AssignStmt):
            value = self.interpreter.global_scope.get(stmt.name)
            typ = checked.expr_types.get(id(stmt.expr), "value")
            return [f"{stmt.name} : {typ} = {render_value(value)}"]
        if isinstance(stmt, FunctionDefStmt):
            signature = checked.function_signatures.get(stmt.name)
            if signature is None:
                return [f"defined {stmt.name}"]
            params = ",".join(signature.param_types)
            return [f"defined {stmt.name} : fn({params}):{signature.return_type}"]
        return []


def is_complete_repl_source(source: str) -> bool:
    if not source.strip():
        return True
    if _has_unclosed_delimiter_or_string(source):
        return False
    try:
        parse_program(source, path="<repl>")
    except AxiomParseError as error:
        return not _parse_error_looks_incomplete(error)
    return True


def _has_unclosed_delimiter_or_string(source: str) -> bool:
    braces = 0
    parens = 0
    brackets = 0
    in_string = False
    escaped = False
    for ch in source:
        if in_string:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
            continue
        if ch == '"':
            in_string = True
        elif ch == "{":
            braces += 1
        elif ch == "}":
            braces -= 1
        elif ch == "(":
            parens += 1
        elif ch == ")":
            parens -= 1
        elif ch == "[":
            brackets += 1
        elif ch == "]":
            brackets -= 1
    return in_string or braces > 0 or parens > 0 or brackets > 0


def _parse_error_looks_incomplete(error: AxiomParseError) -> bool:
    return (
        "got EOF" in error.message
        or "unterminated string literal" in error.message
        or "unterminated escape sequence" in error.message
    )


def run_repl(
    *,
    allow_host_side_effects: bool = False,
    stdin: TextIO | None = None,
    stdout: TextIO | None = None,
    stderr: TextIO | None = None,
) -> int:
    stdin = stdin or sys.stdin
    stdout = stdout or sys.stdout
    stderr = stderr or sys.stderr
    interactive = stdin.isatty()

    install_history()
    if interactive:
        print(INTRO, file=stderr)

    session = ReplSession(allow_host_side_effects=allow_host_side_effects)
    buffer: list[str] = []

    while True:
        prompt = PRIMARY_PROMPT if not buffer else CONTINUATION_PROMPT
        try:
            line = _read_repl_line(stdin, prompt=prompt, interactive=interactive)
        except KeyboardInterrupt:
            buffer = []
            if interactive:
                print("^C", file=stderr)
                continue
            print("error: interrupted", file=stderr)
            return 130
        except EOFError:
            if buffer:
                print("error: incomplete input before EOF", file=stderr)
                return 1
            return 0

        stripped = line.strip()
        if not buffer and stripped in {":quit", ":exit"}:
            return 0
        if not buffer and stripped == ":help":
            print(HELP, file=stdout)
            continue
        if not buffer and stripped.startswith(":"):
            print(f"error: {UNKNOWN_COMMAND} {stripped!r} (try :help)", file=stderr)
            continue
        if not buffer and stripped == "":
            continue

        buffer.append(line.rstrip("\n"))
        source = "\n".join(buffer)
        if not is_complete_repl_source(source):
            continue

        buffer = []
        try:
            result = session.accept(source, stdout)
        except AxiomError as error:
            print(f"error: {error}", file=stderr)
            continue
        for result_line in result.lines:
            print(result_line, file=stdout)


def _read_repl_line(stdin: TextIO, *, prompt: str, interactive: bool) -> str:
    if interactive and stdin is sys.stdin:
        return input(prompt)
    line = stdin.readline()
    if line == "":
        raise EOFError
    return line.rstrip("\n")
