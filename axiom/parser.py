from __future__ import annotations

from typing import List

from .ast import (
    Program,
    LetStmt,
    AssignStmt,
    PrintStmt,
    ExprStmt,
    BlockStmt,
    IfStmt,
    WhileStmt,
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

    def _peek_n(self, n: int) -> Token:
        idx = min(self.i + n, len(self.toks) - 1)
        return self.toks[idx]

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
        if k == TokenKind.LBRACE:
            return self._parse_block()
        if k == TokenKind.IF:
            return self._parse_if()
        if k == TokenKind.WHILE:
            return self._parse_while()
        if k == TokenKind.IDENT and self._peek_n(1).kind == TokenKind.EQ:
            return self._parse_assign()
        return self._parse_expr_stmt()

    def _parse_block(self) -> BlockStmt:
        lbrace = self._eat(TokenKind.LBRACE)
        self._eat_newlines()
        stmts = []
        while self._peek().kind not in (TokenKind.RBRACE, TokenKind.EOF):
            stmts.append(self._parse_stmt())
            self._eat_newlines()
        rbrace = self._eat(TokenKind.RBRACE)
        return BlockStmt(stmts=stmts, span=Span(lbrace.span.start, rbrace.span.end))

    def _parse_if(self) -> IfStmt:
        start = self._eat(TokenKind.IF).span.start
        cond = self._parse_expr()
        then_block = self._parse_block()
        else_block = None
        if self._peek().kind == TokenKind.ELSE:
            self._bump()
            else_block = self._parse_block()
            end = else_block.span.end
        else:
            end = then_block.span.end
        return IfStmt(cond=cond, then_block=then_block, else_block=else_block, span=Span(start, end))

    def _parse_while(self) -> WhileStmt:
        start = self._eat(TokenKind.WHILE).span.start
        cond = self._parse_expr()
        body = self._parse_block()
        return WhileStmt(cond=cond, body=body, span=Span(start, body.span.end))

    def _parse_let(self) -> LetStmt:
        start = self._bump().span.start
        ident = self._bump()
        if ident.kind != TokenKind.IDENT:
            raise AxiomParseError("expected identifier after 'let'", ident.span)
        self._eat(TokenKind.EQ)
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return LetStmt(name=str(ident.value), expr=expr, span=Span(start, end))

    def _parse_assign(self) -> AssignStmt:
        ident = self._eat(TokenKind.IDENT)
        self._eat(TokenKind.EQ)
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return AssignStmt(name=str(ident.value), expr=expr, span=Span(ident.span.start, end))

    def _parse_print(self) -> PrintStmt:
        start = self._bump().span.start
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return PrintStmt(expr=expr, span=Span(start, end))

    def _parse_expr_stmt(self) -> ExprStmt:
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return ExprStmt(expr=expr, span=Span(expr_span(expr).start, end))

    def _parse_terminator(self, default_end: int) -> int:
        k = self._peek().kind
        if k == TokenKind.SEMI:
            return self._bump().span.end
        if k == TokenKind.NEWLINE:
            end = self._bump().span.end
            self._eat_newlines()
            return end
        if k in (TokenKind.EOF, TokenKind.RBRACE):
            return default_end
        raise AxiomParseError("expected ';' or newline", self._peek().span)

    def _parse_expr(self) -> Expr:
        return self._parse_equality()

    def _parse_equality(self) -> Expr:
        node = self._parse_comparison()
        while True:
            k = self._peek().kind
            if k == TokenKind.EQEQ:
                op = BinOp.EQ
            elif k == TokenKind.NE:
                op = BinOp.NE
            else:
                break
            self._bump()
            rhs = self._parse_comparison()
            node = Binary(op=op, lhs=node, rhs=rhs, span=Span(expr_span(node).start, expr_span(rhs).end))
        return node

    def _parse_comparison(self) -> Expr:
        node = self._parse_add_sub()
        while True:
            k = self._peek().kind
            if k == TokenKind.LT:
                op = BinOp.LT
            elif k == TokenKind.LE:
                op = BinOp.LE
            elif k == TokenKind.GT:
                op = BinOp.GT
            elif k == TokenKind.GE:
                op = BinOp.GE
            else:
                break
            self._bump()
            rhs = self._parse_add_sub()
            node = Binary(op=op, lhs=node, rhs=rhs, span=Span(expr_span(node).start, expr_span(rhs).end))
        return node

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
            self._bump()
            rhs = self._parse_mul_div()
            node = Binary(op=op, lhs=node, rhs=rhs, span=Span(expr_span(node).start, expr_span(rhs).end))
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
            self._bump()
            rhs = self._parse_factor()
            node = Binary(op=op, lhs=node, rhs=rhs, span=Span(expr_span(node).start, expr_span(rhs).end))
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
            return UnaryNeg(expr=inner, span=Span(minus.span.start, expr_span(inner).end))
        if t.kind == TokenKind.LPAREN:
            l = self._bump()
            expr = self._parse_expr()
            r = self._eat(TokenKind.RPAREN)
            return _widen_span(expr, Span(l.span.start, r.span.end))
        raise AxiomParseError("expected expression", t.span)


def _widen_span(expr: Expr, span: Span) -> Expr:
    if isinstance(expr, IntLit):
        return IntLit(expr.value, span)
    if isinstance(expr, VarRef):
        return VarRef(expr.name, span)
    if isinstance(expr, UnaryNeg):
        return UnaryNeg(expr=expr.expr, span=span)
    if isinstance(expr, Binary):
        return Binary(op=expr.op, lhs=expr.lhs, rhs=expr.rhs, span=span)
    raise AssertionError("unknown expr type")
