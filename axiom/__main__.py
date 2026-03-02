from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .api import parse_program, compile_to_bytecode
from .bytecode import Bytecode
from .interpreter import Interpreter
from .vm import Vm
from .errors import AxiomError


def cmd_interp(path: Path) -> int:
    src = path.read_text(encoding="utf-8")
    program = parse_program(src)
    interp = Interpreter()
    interp.run(program, sys.stdout)
    return 0


def cmd_compile(path: Path, out_path: Path) -> int:
    src = path.read_text(encoding="utf-8")
    bc = compile_to_bytecode(src)
    out_path.write_bytes(bc.encode())
    print(f"wrote {out_path} ({out_path.stat().st_size} bytes)", file=sys.stderr)
    return 0


def cmd_vm(path: Path) -> int:
    bc = Bytecode.decode(path.read_bytes())
    vm = Vm(locals_count=bc.locals_count)
    vm.run(bc, sys.stdout)
    return 0


def cmd_check(path: Path) -> int:
    src = path.read_text(encoding="utf-8")
    _ = compile_to_bytecode(src)  # compilation does basic semantic checks (e.g., undefined vars)
    print("OK", file=sys.stderr)
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="axiom", description="Axiom language tool (stage0 interpreter + stage1 compiler/VM)")
    sub = p.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("interp", help="Run Axiom source via the interpreter")
    sp.add_argument("file", type=Path)

    sp = sub.add_parser("compile", help="Compile Axiom source to bytecode (.axb)")
    sp.add_argument("file", type=Path)
    sp.add_argument("-o", "--output", required=True, type=Path)

    sp = sub.add_parser("vm", help="Run bytecode on the VM")
    sp.add_argument("file", type=Path)

    sp = sub.add_parser("check", help="Parse + semantic checks (currently: undefined vars via compilation)")
    sp.add_argument("file", type=Path)

    args = p.parse_args(argv)

    try:
        if args.cmd == "interp":
            return cmd_interp(args.file)
        if args.cmd == "compile":
            return cmd_compile(args.file, args.output)
        if args.cmd == "vm":
            return cmd_vm(args.file)
        if args.cmd == "check":
            return cmd_check(args.file)
        raise AssertionError("unreachable")
    except AxiomError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
