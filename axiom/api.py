from __future__ import annotations

from .lexer import Lexer
from .parser import Parser
from .ast import Program
from .compiler import Compiler
from .bytecode import Bytecode


def parse_program(src: str) -> Program:
    toks = Lexer(src).tokenize()
    return Parser(toks).parse_program()


def compile_to_bytecode(src: str, *, allow_host_side_effects: bool = False) -> Bytecode:
    program = parse_program(src)
    return Compiler(allow_host_side_effects=allow_host_side_effects).compile(program)
