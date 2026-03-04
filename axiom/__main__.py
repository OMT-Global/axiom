from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .api import parse_program, compile_to_bytecode
from .bytecode import Bytecode, Op
from .interpreter import Interpreter
from .vm import Vm
from .errors import AxiomError


def cmd_interp(path: Path, *, allow_host_side_effects: bool) -> int:
    src = path.read_text(encoding="utf-8")
    program = parse_program(src)
    Interpreter(allow_host_side_effects=allow_host_side_effects).run(program, sys.stdout)
    return 0


def cmd_compile(path: Path, out_path: Path, *, allow_host_side_effects: bool) -> int:
    src = path.read_text(encoding="utf-8")
    bc = compile_to_bytecode(src, allow_host_side_effects=allow_host_side_effects)
    out_path.write_bytes(bc.encode())
    print(f"wrote {out_path} ({out_path.stat().st_size} bytes)", file=sys.stderr)
    return 0


def cmd_vm(path: Path, *, allow_host_side_effects: bool) -> int:
    bc = Bytecode.decode(path.read_bytes())
    Vm(locals_count=bc.locals_count, allow_host_side_effects=allow_host_side_effects).run(
        bc, sys.stdout
    )
    return 0


def cmd_run(path: Path, *, allow_host_side_effects: bool) -> int:
    src = path.read_text(encoding="utf-8")
    bc = compile_to_bytecode(src, allow_host_side_effects=allow_host_side_effects)
    Vm(locals_count=bc.locals_count, allow_host_side_effects=allow_host_side_effects).run(
        bc, sys.stdout
    )
    return 0


def cmd_disasm(path: Path) -> int:
    bc = Bytecode.decode(path.read_bytes())
    names = {v: k for k, v in Op.__dict__.items() if k.isupper() and isinstance(v, int)}
    for idx, ins in enumerate(bc.instructions):
        name = names.get(ins.op, f"OP_{ins.op}")
        if ins.arg is None:
            print(f"{idx:04d} {name}")
        else:
            print(f"{idx:04d} {name} {ins.arg}")
    return 0


def cmd_check(path: Path, *, allow_host_side_effects: bool) -> int:
    src = path.read_text(encoding="utf-8")
    _ = compile_to_bytecode(src, allow_host_side_effects=allow_host_side_effects)
    print("OK", file=sys.stderr)
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="axiom", description="Axiom language tool (stage0 interpreter + stage1 compiler/VM)")
    sub = p.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("interp", help="Run Axiom source via the interpreter")
    sp.add_argument("file", type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")

    sp = sub.add_parser("compile", help="Compile Axiom source to bytecode (.axb)")
    sp.add_argument("file", type=Path)
    sp.add_argument("-o", "--output", required=True, type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")

    sp = sub.add_parser("vm", help="Run bytecode on the VM")
    sp.add_argument("file", type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")

    sp = sub.add_parser("run", help="Compile source in-memory and execute on VM")
    sp.add_argument("file", type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")

    sp = sub.add_parser("disasm", help="Disassemble bytecode")
    sp.add_argument("file", type=Path)

    sp = sub.add_parser("check", help="Parse + semantic checks (currently: undefined vars via compilation)")
    sp.add_argument("file", type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")

    args = p.parse_args(argv)

    try:
        if args.cmd == "interp":
            return cmd_interp(args.file, allow_host_side_effects=args.allow_host_side_effects)
        if args.cmd == "compile":
            return cmd_compile(
                args.file,
                args.output,
                allow_host_side_effects=args.allow_host_side_effects,
            )
        if args.cmd == "vm":
            return cmd_vm(args.file, allow_host_side_effects=args.allow_host_side_effects)
        if args.cmd == "run":
            return cmd_run(args.file, allow_host_side_effects=args.allow_host_side_effects)
        if args.cmd == "disasm":
            return cmd_disasm(args.file)
        if args.cmd == "check":
            return cmd_check(args.file, allow_host_side_effects=args.allow_host_side_effects)
        raise AssertionError("unreachable")
    except AxiomError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
