from __future__ import annotations

import re
from pathlib import Path
from typing import List, Optional

from .ast import (
    Program,
    LetStmt,
    AssignStmt,
    PrintStmt,
    ReturnStmt,
    FunctionDefStmt,
    ImportStmt,
    ExprStmt,
    BlockStmt,
    IfStmt,
    WhileStmt,
    Expr,
    IntLit,
    StringLit,
    BoolLit,
    VarRef,
    CallExpr,
    UnaryNeg,
    Binary,
    BinOp,
    ArrayLit,
    IndexExpr,
    Param,
    TypeRef,
    expr_span,
)
from .errors import Span, AxiomParseError
from .token import Token, TokenKind


class Parser:
    def __init__(
        self,
        toks: List[Token],
        *,
        source: Optional[str] = None,
        source_path: Optional[str] = None,
    ):
        self.toks = toks
        self.i = 0
        self.function_depth = 0
        self.imported_modules: set[str] = set()
        self.imported_paths: set[str] = set()
        self.source = source
        self.source_path = source_path

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
            raise AxiomParseError(
                f"expected {kind.name}, got {t.kind.name}",
                t.span,
                source=self.source,
                path=self.source_path,
            )
        self.i += 1
        return t

    def _eat_newlines(self) -> None:
        while self._peek().kind == TokenKind.NEWLINE:
            self.i += 1

    def _eat_name_token(self) -> str:
        t = self._peek()
        if t.kind == TokenKind.IDENT:
            self.i += 1
            return str(t.value)
        if t.kind in {
            TokenKind.LET,
            TokenKind.IMPORT,
            TokenKind.FN,
            TokenKind.PRINT,
            TokenKind.RETURN,
            TokenKind.IF,
            TokenKind.ELSE,
            TokenKind.WHILE,
        }:
            self.i += 1
            return t.kind.name.lower()
        raise AxiomParseError(
            "expected identifier",
            t.span,
            source=self.source,
            path=self.source_path,
        )

    def _eat_qualified_name(self) -> str:
        parts = [self._eat_name_token()]
        while self._peek().kind == TokenKind.DOT:
            self._bump()
            parts.append(self._eat_name_token())
        return ".".join(parts)

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
        if k == TokenKind.IMPORT:
            return self._parse_import()
        if k == TokenKind.FN:
            return self._parse_function_def()
        if k == TokenKind.PRINT:
            return self._parse_print()
        if k == TokenKind.RETURN:
            if self.function_depth == 0:
                raise AxiomParseError(
                    "return outside function",
                    self._peek().span,
                    source=self.source,
                    path=self.source_path,
                )
            return self._parse_return()
        if k == TokenKind.LBRACE:
            return self._parse_block()
        if k == TokenKind.IF:
            return self._parse_if()
        if k == TokenKind.WHILE:
            return self._parse_while()
        if k == TokenKind.IDENT and self._peek_n(1).kind == TokenKind.EQ:
            return self._parse_assign()
        return self._parse_expr_stmt()

    def _parse_function_def(self) -> FunctionDefStmt:
        start = self._eat(TokenKind.FN).span.start
        name = self._eat(TokenKind.IDENT)
        if name.kind != TokenKind.IDENT:
            raise AxiomParseError(
                "expected function name",
                name.span,
                source=self.source,
                path=self.source_path,
            )
        if name.value == "host":
            raise AxiomParseError(
                "function name cannot be 'host'",
                name.span,
                source=self.source,
                path=self.source_path,
            )
        self._eat(TokenKind.LPAREN)

        params: List[Param] = []
        if self._peek().kind != TokenKind.RPAREN:
            while True:
                ident = self._eat(TokenKind.IDENT)
                if ident.kind != TokenKind.IDENT:
                    raise AxiomParseError(
                        "expected parameter name",
                        ident.span,
                        source=self.source,
                        path=self.source_path,
                    )
                if ident.value == "host":
                    raise AxiomParseError(
                        "parameter name cannot be 'host'",
                        ident.span,
                        source=self.source,
                        path=self.source_path,
                    )
                type_ref = None
                param_end = ident.span.end
                if self._peek().kind == TokenKind.COLON:
                    self._bump()
                    type_ref = self._parse_type_ref()
                    param_end = type_ref.span.end
                params.append(
                    Param(
                        name=str(ident.value),
                        type_ref=type_ref,
                        span=Span(ident.span.start, param_end),
                    )
                )
                if self._peek().kind == TokenKind.COMMA:
                    self._bump()
                    continue
                break
        self._eat(TokenKind.RPAREN)
        return_type = None
        if self._peek().kind == TokenKind.COLON:
            self._bump()
            return_type = self._parse_type_ref()

        # allow duplicate parameter names only if source author wrote them; catch for deterministic errors.
        param_names = [param.name for param in params]
        if len(set(param_names)) != len(param_names):
            raise AxiomParseError(
                "duplicate function parameter name",
                name.span,
                source=self.source,
                path=self.source_path,
            )

        self.function_depth += 1
        try:
            body = self._parse_block()
        finally:
            self.function_depth -= 1
        return FunctionDefStmt(
            name=str(name.value),
            params=params,
            return_type=return_type,
            body=body,
            span=Span(start, body.span.end),
        )

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
            raise AxiomParseError(
                "expected identifier after 'let'",
                ident.span,
                source=self.source,
                path=self.source_path,
            )
        if ident.value == "host":
            raise AxiomParseError(
                "identifier cannot be 'host'",
                ident.span,
                source=self.source,
                path=self.source_path,
            )
        type_ref = None
        if self._peek().kind == TokenKind.COLON:
            self._bump()
            type_ref = self._parse_type_ref()
        self._eat(TokenKind.EQ)
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return LetStmt(
            name=str(ident.value),
            type_ref=type_ref,
            expr=expr,
            span=Span(start, end),
        )

    def _parse_import(self) -> ImportStmt:
        start = self._eat(TokenKind.IMPORT).span.start
        path = self._eat(TokenKind.STRING)
        if not isinstance(path.value, str):
            raise AxiomParseError(
                "expected import path string",
                path.span,
                source=self.source,
                path=self.source_path,
            )
        if path.value in self.imported_paths:
            raise AxiomParseError(
                "duplicate import path",
                path.span,
                source=self.source,
                path=self.source_path,
            )
        self.imported_paths.add(path.value)
        default_alias = _derive_import_alias(path.value)
        if not default_alias:
            raise AxiomParseError(
                "invalid import path for namespace",
                path.span,
                source=self.source,
                path=self.source_path,
            )
        if default_alias == "host" or default_alias.startswith("host."):
            raise AxiomParseError(
                "import namespace cannot be 'host'",
                path.span,
                source=self.source,
                path=self.source_path,
            )

        alias = default_alias
        if self._peek().kind == TokenKind.AS:
            self._bump()
            alias = self._eat_qualified_name()
            if alias == "host" or alias.startswith("host."):
                raise AxiomParseError(
                    "import namespace cannot be 'host'",
                    path.span,
                    source=self.source,
                    path=self.source_path,
                )
        if alias in self.imported_modules:
            raise AxiomParseError(
                "duplicate import namespace",
                path.span,
                source=self.source,
                path=self.source_path,
            )
        self.imported_modules.add(alias)
        end = self._parse_terminator(default_end=path.span.end)
        return ImportStmt(path=path.value, alias=alias, span=Span(start, end))

    def _parse_assign(self) -> AssignStmt:
        ident = self._eat(TokenKind.IDENT)
        if ident.value == "host":
            raise AxiomParseError(
                "identifier cannot be 'host'",
                ident.span,
                source=self.source,
                path=self.source_path,
            )
        self._eat(TokenKind.EQ)
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return AssignStmt(name=str(ident.value), expr=expr, span=Span(ident.span.start, end))

    def _parse_print(self) -> PrintStmt:
        start = self._bump().span.start
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return PrintStmt(expr=expr, span=Span(start, end))

    def _parse_return(self) -> ReturnStmt:
        start = self._bump().span.start
        expr = self._parse_expr()
        end = self._parse_terminator(default_end=expr_span(expr).end)
        return ReturnStmt(expr=expr, span=Span(start, end))

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
        raise AxiomParseError(
            "expected ';' or newline",
            self._peek().span,
            source=self.source,
            path=self.source_path,
        )

    def _parse_expr(self) -> Expr:
        return self._parse_equality()

    def _parse_type_ref(self) -> TypeRef:
        token = self._peek()
        # Function type: fn(T1, T2, ...): R
        if token.kind == TokenKind.FN:
            start = token.span.start
            self._bump()  # eat 'fn'
            self._eat(TokenKind.LPAREN)
            param_type_names: List[str] = []
            if self._peek().kind != TokenKind.RPAREN:
                while True:
                    pt = self._parse_type_ref()
                    param_type_names.append(pt.name)
                    if self._peek().kind == TokenKind.COMMA:
                        self._bump()
                        continue
                    break
            self._eat(TokenKind.RPAREN)
            self._eat(TokenKind.COLON)
            return_type_ref = self._parse_type_ref()
            fn_type_str = "fn(" + ",".join(param_type_names) + "):" + return_type_ref.name
            return TypeRef(name=fn_type_str, span=Span(start, return_type_ref.span.end))
        # Simple type: int, string, bool (with optional [] suffix)
        if token.kind != TokenKind.IDENT:
            raise AxiomParseError(
                "expected type name",
                token.span,
                source=self.source,
                path=self.source_path,
            )
        self._bump()
        raw_name = str(token.value)
        if raw_name not in {"int", "string", "bool"}:
            raise AxiomParseError(
                f"unknown type {raw_name!r}",
                token.span,
                source=self.source,
                path=self.source_path,
            )
        # Array type suffix: int[], string[], bool[]
        if self._peek().kind == TokenKind.LBRACKET:
            self._bump()  # eat [
            rbracket = self._eat(TokenKind.RBRACKET)
            array_name = f"{raw_name}[]"
            return TypeRef(name=array_name, span=Span(token.span.start, rbracket.span.end))
        return TypeRef(name=raw_name, span=token.span)

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
        node = self._parse_postfix()
        while True:
            k = self._peek().kind
            if k == TokenKind.STAR:
                op = BinOp.MUL
            elif k == TokenKind.SLASH:
                op = BinOp.DIV
            else:
                break
            self._bump()
            rhs = self._parse_postfix()
            node = Binary(op=op, lhs=node, rhs=rhs, span=Span(expr_span(node).start, expr_span(rhs).end))
        return node

    def _parse_postfix(self) -> Expr:
        node = self._parse_primary()
        while self._peek().kind == TokenKind.LBRACKET:
            self._bump()  # eat [
            self._eat_newlines()
            index = self._parse_expr()
            self._eat_newlines()
            rbracket = self._eat(TokenKind.RBRACKET)
            node = IndexExpr(array=node, index=index, span=Span(expr_span(node).start, rbracket.span.end))
        return node

    def _parse_primary(self) -> Expr:
        t = self._peek()
        if t.kind == TokenKind.INT:
            tok = self._bump()
            return IntLit(int(tok.value), tok.span)
        if t.kind == TokenKind.STRING:
            tok = self._bump()
            return StringLit(str(tok.value), tok.span)
        if t.kind in (TokenKind.TRUE, TokenKind.FALSE):
            tok = self._bump()
            return BoolLit(bool(tok.value), tok.span)
        if t.kind == TokenKind.IDENT:
            tok = self._bump()
            callee_parts = [str(tok.value)]
            while self._peek().kind == TokenKind.DOT:
                self._bump()
                callee_parts.append(self._eat_name_token())
            callee = ".".join(callee_parts)
            if "." in callee and not callee.startswith("host."):
                module_name, sep, _fn_name = callee.rpartition(".")
                if module_name not in self.imported_modules:
                    raise AxiomParseError(
                        "only host or imported module calls are supported",
                        tok.span,
                        source=self.source,
                        path=self.source_path,
                    )
            if self._peek().kind == TokenKind.LPAREN:
                self._bump()
                args = []
                if self._peek().kind != TokenKind.RPAREN:
                    while True:
                        args.append(self._parse_expr())
                        if self._peek().kind == TokenKind.COMMA:
                            self._bump()
                            continue
                        break
                rparen = self._eat(TokenKind.RPAREN)
                return CallExpr(callee=callee, args=args, span=Span(tok.span.start, rparen.span.end))
            if "." in callee:
                raise AxiomParseError(
                    "call expected after dotted name",
                    t.span,
                    source=self.source,
                    path=self.source_path,
                )
            return VarRef(str(tok.value), tok.span)
        if t.kind == TokenKind.MINUS:
            minus = self._bump()
            inner = self._parse_postfix()
            return UnaryNeg(expr=inner, span=Span(minus.span.start, expr_span(inner).end))
        if t.kind == TokenKind.LPAREN:
            lparen_token = self._bump()
            expr = self._parse_expr()
            rparen_token = self._eat(TokenKind.RPAREN)
            return _widen_span(expr, Span(lparen_token.span.start, rparen_token.span.end))
        if t.kind == TokenKind.LBRACKET:
            lbracket = self._bump()  # eat [
            elements: List[Expr] = []
            self._eat_newlines()
            if self._peek().kind != TokenKind.RBRACKET:
                while True:
                    elements.append(self._parse_expr())
                    self._eat_newlines()
                    if self._peek().kind == TokenKind.COMMA:
                        self._bump()
                        self._eat_newlines()
                        if self._peek().kind == TokenKind.RBRACKET:
                            break  # trailing comma
                        continue
                    break
            self._eat_newlines()
            rbracket = self._eat(TokenKind.RBRACKET)
            return ArrayLit(elements=elements, span=Span(lbracket.span.start, rbracket.span.end))
        raise AxiomParseError(
            "expected expression",
            t.span,
            source=self.source,
            path=self.source_path,
        )

    # Keep _parse_factor as an alias so external callers aren't broken.
    _parse_factor = _parse_primary


_NON_ID_CHARS = re.compile(r"[^0-9A-Za-z_]")


def _normalize_identifier_part(part: str) -> str:
    normalized = _NON_ID_CHARS.sub("_", part)
    if not normalized:
        return ""
    if normalized[0].isdigit():
        normalized = f"m_{normalized}"
    return normalized


def _derive_import_alias(raw_path: str) -> str:
    path = Path(raw_path)
    no_ext = path.with_suffix("")
    parts = [p for p in no_ext.parts if p not in (".", "")]
    if not parts:
        return ""
    normalized_parts = [_normalize_identifier_part(part) for part in parts if part not in ("",)]
    return ".".join(part for part in normalized_parts if part)


def _widen_span(expr: Expr, span: Span) -> Expr:
    if isinstance(expr, IntLit):
        return IntLit(expr.value, span)
    if isinstance(expr, StringLit):
        return StringLit(expr.value, span)
    if isinstance(expr, BoolLit):
        return BoolLit(expr.value, span)
    if isinstance(expr, VarRef):
        return VarRef(expr.name, span)
    if isinstance(expr, CallExpr):
        return CallExpr(callee=expr.callee, args=expr.args, span=span)
    if isinstance(expr, UnaryNeg):
        return UnaryNeg(expr=expr.expr, span=span)
    if isinstance(expr, Binary):
        return Binary(op=expr.op, lhs=expr.lhs, rhs=expr.rhs, span=span)
    if isinstance(expr, ArrayLit):
        return ArrayLit(elements=expr.elements, span=span)
    if isinstance(expr, IndexExpr):
        return IndexExpr(array=expr.array, index=expr.index, span=span)
    raise AssertionError("unknown expr type")
