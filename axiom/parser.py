from __future__ import annotations

from typing import List

from .ast import (
    Program,
    LetStmt,
    PrintStmt,
    ExprStmt,
    Expr,
    IntLit,
    VarRef,
    UnaryNeg,
    Binary,
    BinOp,
    expr_span,
)
from .errors import Span, AxiomParseError
from .token import Token, TokenKind


class Parser:
    def __init__(self, toks: List[Token]):
        self.toks = toks
        self.i = 0

    def _peek(self) -> Token:
        return self.toks[self.i]

    def _bump(self) -> Token:
        t = self._peek()
        self.i += 1
        return t

    def _eat(self, kind: TokenKind) -> Token:
        t = self._peek()
        if t.kind != kind:
            raise AxiomParseError(f"expected {kind.name}, got {t.kind.name}", t.span)
        self.i += 1
        return t

    def _eat_newlines(self) -> None:
        while self._peek().kind == TokenKind.NEWLINE:
            self.i += 1

    def parse_program(self) -> Program:
        stmts = []
        self._eat_newlines()
        while self._peek().kind != TokenKind.EOF:
            stmts.append(self._parse_stmt())
            self._eat_newlines()
        return Program(stmts)

    def _parse_stmt(self):
        k = self._peek().kind
        if k == TokenKind.LET:
            return self._parse_let()
        if k == TokenKind.PRINT:
            return self._parse_print()
        return self._parse_expr_stmt()

    def _parse_let(self) -> LetStmt:
        start = self._bump().span.start  # let
        ident = self._bump()
        if ident.kind != TokenKind.IDENT:
            raise AxiomParseError("expected identifier after 'let'", ident.span)
        name = str(ident.value)
        self._eat(TokenKind.EQ)
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return LetStmt(name=name, expr=expr, span=Span(start, end))

    def _parse_print(self) -> PrintStmt:
        start = self._bump().span.start  # print
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return PrintStmt(expr=expr, span=Span(start, end))

    def _parse_expr_stmt(self) -> ExprStmt:
        expr = self._parse_expr()
        start = expr_span(expr).start
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return ExprStmt(expr=expr, span=Span(start, end))

    def _parse_terminator(self, default_end: int) -> int:
        k = self._peek().kind
        if k == TokenKind.SEMI:
            return self._bump().span.end
        if k == TokenKind.NEWLINE:
            end = self._bump().span.end
            self._eat_newlines()
            return end
        if k == TokenKind.EOF:
            return default_end
        t = self._peek()
        raise AxiomParseError("expected ';' or newline", t.span)

    def _parse_expr(self) -> Expr:
        return self._parse_add_sub()

    def _parse_add_sub(self) -> Expr:
        node = self._parse_mul_div()
        while True:
            k = self._peek().kind
            if k == TokenKind.PLUS:
                op = BinOp.ADD
            elif k == TokenKind.MINUS:
                op = BinOp.SUB
            else:
                break
            op_tok = self._bump()
            rhs = self._parse_mul_div()
            span = Span(expr_span(node).start, expr_span(rhs).end)
            node = Binary(op=op, lhs=node, rhs=rhs, span=span)
        return node

    def _parse_mul_div(self) -> Expr:
        node = self._parse_factor()
        while True:
            k = self._peek().kind
            if k == TokenKind.STAR:
                op = BinOp.MUL
            elif k == TokenKind.SLASH:
                op = BinOp.DIV
            else:
                break
            op_tok = self._bump()
            rhs = self._parse_factor()
            span = Span(expr_span(node).start, expr_span(rhs).end)
            node = Binary(op=op, lhs=node, rhs=rhs, span=span)
        return node

    def _parse_factor(self) -> Expr:
        t = self._peek()
        if t.kind == TokenKind.INT:
            tok = self._bump()
            return IntLit(int(tok.value), tok.span)
        if t.kind == TokenKind.IDENT:
            tok = self._bump()
            return VarRef(str(tok.value), tok.span)
        if t.kind == TokenKind.MINUS:
            minus = self._bump()
            inner = self._parse_factor()
            span = Span(minus.span.start, expr_span(inner).end)
            return UnaryNeg(expr=inner, span=span)
        if t.kind == TokenKind.LPAREN:
            l = self._bump()
            expr = self._parse_expr()
            r = self._eat(TokenKind.RPAREN)
            # widen span to include parentheses
            widened = Span(l.span.start, r.span.end)
            return _widen_span(expr, widened)
        raise AxiomParseError("expected expression", t.span)


def _widen_span(expr: Expr, span: Span) -> Expr:
    # Rebuild node with wider span (keeps everything else identical).
    if isinstance(expr, IntLit):
        return IntLit(expr.value, span)
    if isinstance(expr, VarRef):
        return VarRef(expr.name, span)
    if isinstance(expr, UnaryNeg):
        return UnaryNeg(expr=expr.expr, span=span)
    if isinstance(expr, Binary):
        return Binary(op=expr.op, lhs=expr.lhs, rhs=expr.rhs, span=span)
    raise AssertionError("unknown expr type")
