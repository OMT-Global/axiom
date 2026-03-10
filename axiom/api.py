from __future__ import annotations

from pathlib import Path
from typing import Optional, Sequence, Set

from .ast import Program
from .checker import CheckedProgram, check_program
from .compiler import Compiler
from .bytecode import Bytecode
from .loader import load_program_file, parse_program


def parse_file(
    path: Path, module_search_paths: Optional[Sequence[Path]] = None
) -> Program:
    return load_program_file(path, module_search_paths=module_search_paths)


def compile_to_bytecode(
    src: str,
    *,
    allow_host_side_effects: bool = False,
    allowed_host_calls: Optional[Set[str]] = None,
) -> Bytecode:
    program = parse_program(src, path=None)
    checked = check_program(
        program,
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    )
    return Compiler(
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    ).compile(checked)


def compile_file(
    path: Path,
    *,
    allow_host_side_effects: bool = False,
    allowed_host_calls: Optional[Set[str]] = None,
    module_search_paths: Optional[Sequence[Path]] = None,
) -> Bytecode:
    program = parse_file(path, module_search_paths=module_search_paths)
    checked = check_program(
        program,
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    )
    return Compiler(
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    ).compile(checked)


def check_file(
    path: Path,
    *,
    allow_host_side_effects: bool = False,
    allowed_host_calls: Optional[Set[str]] = None,
    module_search_paths: Optional[Sequence[Path]] = None,
) -> CheckedProgram:
    program = parse_file(path, module_search_paths=module_search_paths)
    return check_program(
        program,
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=allowed_host_calls,
    )
