from __future__ import annotations

from typing import List, Optional

from .errors import Span, AxiomParseError
from .token import Token, TokenKind


class Lexer:
    def __init__(self, src: str, path: Optional[str] = None):
        self.src = src
        self.path = path
        self.i = 0
        self.n = len(src)

    def _peek(self) -> Optional[str]:
        if self.i >= self.n:
            return None
        return self.src[self.i]

    def _peek_next(self) -> Optional[str]:
        if self.i + 1 >= self.n:
            return None
        return self.src[self.i + 1]

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
            raise AxiomParseError(
                f"invalid int literal {text!r}: {e}",
                Span(start, end),
                source=self.src,
                path=self.path,
            )
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
        if text == "import":
            return Token(TokenKind.IMPORT, Span(start, end))
        if text == "as":
            return Token(TokenKind.AS, Span(start, end))
        if text == "fn":
            return Token(TokenKind.FN, Span(start, end))
        if text == "print":
            return Token(TokenKind.PRINT, Span(start, end))
        if text == "return":
            return Token(TokenKind.RETURN, Span(start, end))
        if text == "if":
            return Token(TokenKind.IF, Span(start, end))
        if text == "else":
            return Token(TokenKind.ELSE, Span(start, end))
        if text == "while":
            return Token(TokenKind.WHILE, Span(start, end))
        if text == "true":
            return Token(TokenKind.TRUE, Span(start, end), True)
        if text == "false":
            return Token(TokenKind.FALSE, Span(start, end), False)
        return Token(TokenKind.IDENT, Span(start, end), text)

    def _lex_string(self, start: int) -> Token:
        chars: List[str] = []
        while True:
            ch = self._peek()
            if ch is None:
                raise AxiomParseError(
                    "unterminated string literal",
                    Span(start, self.i),
                    source=self.src,
                    path=self.path,
                )
            self.i += 1
            if ch == '"':
                break
            if ch == "\\":
                esc = self._peek()
                if esc is None:
                    raise AxiomParseError(
                        "unterminated escape sequence",
                        Span(start, self.i),
                        source=self.src,
                        path=self.path,
                    )
                self.i += 1
                if esc == "n":
                    chars.append("\n")
                elif esc == "r":
                    chars.append("\r")
                elif esc == "t":
                    chars.append("\t")
                elif esc == "\\":
                    chars.append("\\")
                elif esc == '"':
                    chars.append('"')
                else:
                    chars.append(esc)
            else:
                chars.append(ch)
        end = self.i
        return Token(TokenKind.STRING, Span(start, end), "".join(chars))

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
            if self._peek() == "=":
                self.i += 1
                return Token(TokenKind.EQEQ, Span(start, start + 2))
            return Token(TokenKind.EQ, Span(start, start + 1))
        if ch == "!":
            if self._peek() == "=":
                self.i += 1
                return Token(TokenKind.NE, Span(start, start + 2))
            raise AxiomParseError(
                "unexpected character '!'",
                Span(start, start + 1),
                source=self.src,
                path=self.path,
            )
        if ch == "<":
            if self._peek() == "=":
                self.i += 1
                return Token(TokenKind.LE, Span(start, start + 2))
            return Token(TokenKind.LT, Span(start, start + 1))
        if ch == ">":
            if self._peek() == "=":
                self.i += 1
                return Token(TokenKind.GE, Span(start, start + 2))
            return Token(TokenKind.GT, Span(start, start + 1))
        if ch == "+":
            return Token(TokenKind.PLUS, Span(start, start + 1))
        if ch == "-":
            return Token(TokenKind.MINUS, Span(start, start + 1))
        if ch == "*":
            return Token(TokenKind.STAR, Span(start, start + 1))
        if ch == "/":
            return Token(TokenKind.SLASH, Span(start, start + 1))
        if ch == ".":
            return Token(TokenKind.DOT, Span(start, start + 1))
        if ch == ",":
            return Token(TokenKind.COMMA, Span(start, start + 1))
        if ch == ":":
            return Token(TokenKind.COLON, Span(start, start + 1))
        if ch == '"':
            return self._lex_string(start)
        if ch == "(":
            return Token(TokenKind.LPAREN, Span(start, start + 1))
        if ch == ")":
            return Token(TokenKind.RPAREN, Span(start, start + 1))
        if ch == "{":
            return Token(TokenKind.LBRACE, Span(start, start + 1))
        if ch == "}":
            return Token(TokenKind.RBRACE, Span(start, start + 1))
        if ch == "[":
            return Token(TokenKind.LBRACKET, Span(start, start + 1))
        if ch == "]":
            return Token(TokenKind.RBRACKET, Span(start, start + 1))

        if ch == "#":
            # comment to end of line (don't consume newline)
            while True:
                nxt = self._peek()
                if nxt is None or nxt == "\n":
                    break
                self.i += 1
            return self.next_token()

        if ch.isdigit():
            self.i -= 1
            return self._lex_number(start)

        if self._is_ident_start(ch):
            self.i -= 1
            return self._lex_ident_or_kw(start)

        raise AxiomParseError(
            f"unexpected character {ch!r}",
            Span(start, start + 1),
            source=self.src,
            path=self.path,
        )

    def tokenize(self) -> List[Token]:
        toks: List[Token] = []
        while True:
            t = self.next_token()
            toks.append(t)
            if t.kind == TokenKind.EOF:
                break
        return toks
