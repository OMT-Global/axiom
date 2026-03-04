from __future__ import annotations

import argparse
import sys
from pathlib import Path
import json

from .api import parse_file, compile_file
from .bytecode import Bytecode, Op
from .interpreter import Interpreter
from .vm import Vm
from .errors import AxiomError
from .host import HOST_BUILTINS, HOST_BUILTIN_NAMES
from .packaging import (
    build_package,
    clean_package,
    init_package,
    load_manifest,
    manifest_to_dict,
)


def cmd_interp(path: Path, *, allow_host_side_effects: bool) -> int:
    program = parse_file(path)
    Interpreter(allow_host_side_effects=allow_host_side_effects).run(program, sys.stdout)
    return 0


def cmd_compile(path: Path, out_path: Path, *, allow_host_side_effects: bool) -> int:
    bc = compile_file(path, allow_host_side_effects=allow_host_side_effects)
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
    bc = compile_file(path, allow_host_side_effects=allow_host_side_effects)
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
    _ = compile_file(path, allow_host_side_effects=allow_host_side_effects)
    print("OK", file=sys.stderr)
    return 0


def cmd_pkg_init(
    path: Path,
    *,
    name: str | None = None,
    version: str | None = None,
    main: str | None = None,
    out_dir: str | None = None,
    output: str | None = None,
    force: bool = False,
) -> int:
    manifest = init_package(
        path,
        name=name,
        version=version,
        main=main,
        out_dir=out_dir,
        output=output,
        force=force,
    )
    print(f"initialized package {manifest.name} in {path}", file=sys.stderr)
    return 0


def cmd_pkg_build(
    path: Path, *, allow_host_side_effects: bool, output: str | None = None
) -> int:
    out_path = build_package(
        path,
        allow_host_side_effects=allow_host_side_effects,
        output=output,
    )
    print(f"wrote {out_path} ({out_path.stat().st_size} bytes)", file=sys.stderr)
    return 0


def cmd_pkg_manifest(path: Path) -> int:
    manifest = load_manifest(path)
    print(json.dumps(manifest_to_dict(manifest), indent=2))
    return 0


def cmd_pkg_check(path: Path, *, allow_host_side_effects: bool) -> int:
    manifest = load_manifest(path)
    entry = path.resolve() / manifest.main
    _ = compile_file(entry, allow_host_side_effects=allow_host_side_effects)
    print("OK", file=sys.stderr)
    return 0


def cmd_pkg_clean(path: Path) -> int:
    removed = clean_package(path)
    if removed:
        print(f"removed package artifacts in {path}")
    else:
        print(f"nothing to clean for package in {path}")
    return 0


def cmd_pkg_run(path: Path, *, allow_host_side_effects: bool) -> int:
    project_root = path.resolve()
    manifest = load_manifest(project_root)
    entry = project_root / manifest.main
    bytecode = compile_file(entry, allow_host_side_effects=allow_host_side_effects)
    Vm(
        locals_count=bytecode.locals_count,
        allow_host_side_effects=allow_host_side_effects,
    ).run(bytecode, sys.stdout)
    return 0


def cmd_host_list() -> int:
    payload = []
    for name in HOST_BUILTIN_NAMES:
        arity, side_effecting = HOST_BUILTINS[name]
        payload.append(
            {"name": name, "arity": arity, "side_effecting": side_effecting}
        )
    print(json.dumps(payload, indent=2))
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

    sp = sub.add_parser("pkg", help="Package helpers")
    pkg = sp.add_subparsers(dest="pkg_cmd", required=True)
    sp_init = pkg.add_parser("init", help="Create a package manifest and default source entry")
    sp_init.add_argument("path", type=Path, default=Path("."), nargs="?")
    sp_init.add_argument("--name")
    sp_init.add_argument("--version", default=None)
    sp_init.add_argument("--main", default=None)
    sp_init.add_argument("--out-dir", default=None)
    sp_init.add_argument("--output", default=None)
    sp_init.add_argument("--force", action="store_true")
    sp_build = pkg.add_parser("build", help="Build package bytecode")
    sp_build.add_argument("path", type=Path, default=Path("."), nargs="?")
    sp_build.add_argument("--allow-host-side-effects", action="store_true")
    sp_build.add_argument("--output", default=None)
    sp_run = pkg.add_parser("run", help="Run package main source via manifest")
    sp_run.add_argument("path", type=Path, default=Path("."), nargs="?")
    sp_run.add_argument("--allow-host-side-effects", action="store_true")
    pkg.add_parser("manifest", help="Print package manifest JSON").add_argument(
        "path", type=Path, default=Path("."), nargs="?"
    )
    sp_check = pkg.add_parser("check", help="Check package manifest and compile main")
    sp_check.add_argument("path", type=Path, default=Path("."), nargs="?")
    sp_check.add_argument("--allow-host-side-effects", action="store_true")
    pkg.add_parser("clean", help="Delete package artifacts").add_argument(
        "path", type=Path, default=Path("."), nargs="?"
    )
    sp_host = sub.add_parser("host", help="Host bridge helpers")
    host = sp_host.add_subparsers(dest="host_cmd", required=True)
    host.add_parser("list", help="List registered host capabilities")

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
        if args.cmd == "pkg":
            if args.pkg_cmd == "init":
                return cmd_pkg_init(
                    args.path,
                    name=args.name,
                    version=args.version,
                    main=args.main,
                    out_dir=args.out_dir,
                    output=args.output,
                    force=args.force,
                )
            if args.pkg_cmd == "build":
                return cmd_pkg_build(
                    args.path,
                    allow_host_side_effects=args.allow_host_side_effects,
                    output=args.output,
                )
            if args.pkg_cmd == "manifest":
                return cmd_pkg_manifest(args.path)
            if args.pkg_cmd == "check":
                return cmd_pkg_check(args.path, allow_host_side_effects=args.allow_host_side_effects)
            if args.pkg_cmd == "clean":
                return cmd_pkg_clean(args.path)
            if args.pkg_cmd == "run":
                return cmd_pkg_run(args.path, allow_host_side_effects=args.allow_host_side_effects)
            raise AssertionError("unreachable")
        if args.cmd == "host":
            if args.host_cmd == "list":
                return cmd_host_list()
            raise AssertionError("unreachable")
        raise AssertionError("unreachable")
    except AxiomError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
