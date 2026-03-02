from __future__ import annotations

from typing import List, Optional

from .errors import Span, AxiomParseError
from .token import Token, TokenKind


class Lexer:
    def __init__(self, src: str):
        self.src = src
        self.i = 0
        self.n = len(src)

    def _peek(self) -> Optional[str]:
        if self.i >= self.n:
            return None
        return self.src[self.i]

    def _bump(self) -> Optional[str]:
        ch = self._peek()
        if ch is None:
            return None
        self.i += 1
        return ch

    @staticmethod
    def _is_ident_start(ch: str) -> bool:
        return ch.isalpha() or ch == "_"

    @staticmethod
    def _is_ident_continue(ch: str) -> bool:
        return ch.isalnum() or ch == "_"

    def _skip_spaces(self) -> None:
        while True:
            ch = self._peek()
            if ch is None:
                return
            if ch in (" ", "\t", "\r"):
                self.i += 1
                continue
            return

    def _lex_number(self, start: int) -> Token:
        while True:
            ch = self._peek()
            if ch is not None and ch.isdigit():
                self.i += 1
            else:
                break
        end = self.i
        text = self.src[start:end]
        try:
            v = int(text, 10)
        except ValueError as e:
            raise AxiomParseError(f"invalid int literal {text!r}: {e}", Span(start, end))
        return Token(TokenKind.INT, Span(start, end), v)

    def _lex_ident_or_kw(self, start: int) -> Token:
        while True:
            ch = self._peek()
            if ch is not None and self._is_ident_continue(ch):
                self.i += 1
            else:
                break
        end = self.i
        text = self.src[start:end]
        if text == "let":
            return Token(TokenKind.LET, Span(start, end))
        if text == "print":
            return Token(TokenKind.PRINT, Span(start, end))
        return Token(TokenKind.IDENT, Span(start, end), text)

    def next_token(self) -> Token:
        self._skip_spaces()
        start = self.i
        ch = self._bump()
        if ch is None:
            return Token(TokenKind.EOF, Span(self.i, self.i))

        if ch == "\n":
            return Token(TokenKind.NEWLINE, Span(start, start + 1))
        if ch == ";":
            return Token(TokenKind.SEMI, Span(start, start + 1))
        if ch == "=":
            return Token(TokenKind.EQ, Span(start, start + 1))
        if ch == "+":
            return Token(TokenKind.PLUS, Span(start, start + 1))
        if ch == "-":
            return Token(TokenKind.MINUS, Span(start, start + 1))
        if ch == "*":
            return Token(TokenKind.STAR, Span(start, start + 1))
        if ch == "/":
            return Token(TokenKind.SLASH, Span(start, start + 1))
        if ch == "(":
            return Token(TokenKind.LPAREN, Span(start, start + 1))
        if ch == ")":
            return Token(TokenKind.RPAREN, Span(start, start + 1))

        if ch == "#":
            # comment to end of line (don't consume newline)
            while True:
                nxt = self._peek()
                if nxt is None or nxt == "\n":
                    break
                self.i += 1
            return self.next_token()

        if ch.isdigit():
            # rewind one char and lex full number
            self.i -= 1
            return self._lex_number(start)

        if self._is_ident_start(ch):
            self.i -= 1
            return self._lex_ident_or_kw(start)

        raise AxiomParseError(f"unexpected character {ch!r}", Span(start, start + 1))

    def tokenize(self) -> List[Token]:
        toks: List[Token] = []
        while True:
            t = self.next_token()
            toks.append(t)
            if t.kind == TokenKind.EOF:
                break
        return toks
