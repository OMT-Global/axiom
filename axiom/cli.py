from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from .api import check_file, compile_file
from .bytecode import VERSION_MAJOR, Bytecode, Op
from .errors import AxiomError
from .host import host_capabilities, host_contract_metadata
from .interpreter import Interpreter
from .packaging import (
    build_package,
    check_package,
    clean_package,
    init_package,
    load_manifest,
    manifest_to_dict,
    run_package,
)
from .repl import run_repl
from .vm import Vm


def _module_search_paths(values: list[str] | None) -> list[Path] | None:
    if not values:
        return None
    return [Path(p) for p in values]


def _emit_json(payload: dict[str, object]) -> None:
    print(json.dumps(payload, sort_keys=True))


def _error_payload(error: AxiomError, args: argparse.Namespace) -> dict[str, object]:
    payload = error.to_dict()
    location = payload.get("location")
    if isinstance(location, dict) and location.get("path") is None and hasattr(args, "file"):
        location["path"] = str(Path(args.file).resolve())
    return payload


def _compile_metadata(bytecode: Bytecode) -> dict[str, object]:
    return {
        "version_major": VERSION_MAJOR,
        "version_minor": bytecode.version_minor,
        "locals_count": bytecode.locals_count,
        "function_count": len(bytecode.functions),
        "module_count": len(bytecode.modules),
        "string_count": len(bytecode.strings),
        "instruction_count": len(bytecode.instructions),
    }


def cmd_interp(
    path: Path, *, allow_host_side_effects: bool, module_paths: list[Path] | None = None
) -> int:
    checked = check_file(
        path,
        allow_host_side_effects=allow_host_side_effects,
        module_search_paths=module_paths,
    )
    Interpreter(allow_host_side_effects=allow_host_side_effects).run(
        checked.program,
        sys.stdout,
    )
    return 0


def cmd_compile(
    path: Path,
    out_path: Path,
    *,
    allow_host_side_effects: bool,
    json_output: bool = False,
    module_paths: list[Path] | None = None,
) -> int:
    bc = compile_file(
        path,
        allow_host_side_effects=allow_host_side_effects,
        module_search_paths=module_paths,
    )
    out_path.write_bytes(bc.encode())
    byte_count = out_path.stat().st_size
    if json_output:
        _emit_json(
            {
                "ok": True,
                "command": "compile",
                "file": str(path),
                "output": str(out_path),
                "bytes": byte_count,
                "bytecode": _compile_metadata(bc),
            }
        )
    else:
        print(f"wrote {out_path} ({byte_count} bytes)", file=sys.stderr)
    return 0


def cmd_vm(path: Path, *, allow_host_side_effects: bool) -> int:
    bc = Bytecode.decode(path.read_bytes())
    Vm(locals_count=bc.locals_count, allow_host_side_effects=allow_host_side_effects).run(
        bc, sys.stdout
    )
    return 0


def cmd_run(
    path: Path, *, allow_host_side_effects: bool, module_paths: list[Path] | None = None
) -> int:
    bc = compile_file(
        path,
        allow_host_side_effects=allow_host_side_effects,
        module_search_paths=module_paths,
    )
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
        elif ins.op == Op.CONST_BOOL:
            print(f"{idx:04d} {name} {'true' if int(ins.arg) else 'false'}")
        elif ins.op == Op.CONST_STRING:
            try:
                value = bc.strings[int(ins.arg)]
            except (IndexError, TypeError, ValueError):
                value = "<invalid-string-index>"
            print(f"{idx:04d} {name} {ins.arg} {value!r}")
        else:
            print(f"{idx:04d} {name} {ins.arg}")
    return 0


def cmd_check(
    path: Path,
    *,
    allow_host_side_effects: bool,
    json_output: bool = False,
    module_paths: list[Path] | None = None,
) -> int:
    checked = check_file(
        path,
        allow_host_side_effects=allow_host_side_effects,
        module_search_paths=module_paths,
    )
    if json_output:
        _emit_json(
            {
                "ok": True,
                "command": "check",
                "file": str(path),
                "bytecode_ready": True,
                "functions": len(checked.function_signatures),
                "diagnostics": [],
            }
        )
    else:
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
    allowed_host_calls: list[str] | None = None,
    force: bool = False,
) -> int:
    manifest = init_package(
        path,
        name=name,
        version=version,
        main=main,
        out_dir=out_dir,
        output=output,
        allowed_host_calls=allowed_host_calls,
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
    _ = check_package(path, allow_host_side_effects=allow_host_side_effects)
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
    _ = run_package(path, allow_host_side_effects=allow_host_side_effects, out=sys.stdout)
    return 0


def cmd_host_list(*, safe_only: bool = False, compact: bool = False) -> int:
    payload = host_capabilities(safe_only=safe_only)
    if compact:
        print(json.dumps(payload, sort_keys=True, separators=(",", ":")))
    else:
        print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def cmd_host_describe(*, safe_only: bool = False, compact: bool = False) -> int:
    payload = host_contract_metadata(safe_only=safe_only)
    if compact:
        print(json.dumps(payload, sort_keys=True, separators=(",", ":")))
    else:
        print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="axiom",
        description="Axiom language tool (stage0 interpreter + stage1 compiler/VM)",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("interp", help="Run Axiom source via the interpreter")
    sp.add_argument("file", type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")
    sp.add_argument("--module-path", action="append", default=None)

    sp = sub.add_parser("repl", help="Start an interactive Axiom REPL")
    sp.add_argument("--allow-host-side-effects", action="store_true")

    sp = sub.add_parser("compile", help="Compile Axiom source to bytecode (.axb)")
    sp.add_argument("file", type=Path)
    sp.add_argument("-o", "--output", required=True, type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")
    sp.add_argument("--json", action="store_true")
    sp.add_argument("--module-path", action="append", default=None)

    sp = sub.add_parser("vm", help="Run bytecode on the VM")
    sp.add_argument("file", type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")

    sp = sub.add_parser("run", help="Compile source in-memory and execute on VM")
    sp.add_argument("file", type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")
    sp.add_argument("--module-path", action="append", default=None)

    sp = sub.add_parser("disasm", help="Disassemble bytecode")
    sp.add_argument("file", type=Path)

    sp = sub.add_parser(
        "check",
        help="Parse + type-check source without writing bytecode artifacts",
    )
    sp.add_argument("file", type=Path)
    sp.add_argument("--allow-host-side-effects", action="store_true")
    sp.add_argument("--json", action="store_true")
    sp.add_argument("--module-path", action="append", default=None)

    sp = sub.add_parser("pkg", help="Package helpers")
    pkg = sp.add_subparsers(dest="pkg_cmd", required=True)
    sp_init = pkg.add_parser("init", help="Create a package manifest and default source entry")
    sp_init.add_argument("path", type=Path, default=Path("."), nargs="?")
    sp_init.add_argument("--name")
    sp_init.add_argument("--version", default=None)
    sp_init.add_argument("--main", default=None)
    sp_init.add_argument("--out-dir", default=None)
    sp_init.add_argument("--output", default=None)
    sp_init.add_argument(
        "--allowed-host-call",
        action="append",
        default=None,
        metavar="HOSTCALL",
        help="Allowlist host call (e.g. print, abs, math.abs)",
    )
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
    host_list = host.add_parser("list", help="List registered host capabilities")
    host_list.add_argument(
        "--safe-only",
        action="store_true",
        help="Show only non-side-effecting host functions",
    )
    host_list.add_argument(
        "--compact",
        action="store_true",
        help="Print compact JSON for machine parsing",
    )
    host_desc = host.add_parser("describe", help="Describe host contract for tooling")
    host_desc.add_argument(
        "--safe-only",
        action="store_true",
        help="Include only non-side-effecting host functions",
    )
    host_desc.add_argument(
        "--compact",
        action="store_true",
        help="Print compact JSON for machine parsing",
    )

    return parser


def run_cli(args: argparse.Namespace) -> int:
    if args.cmd == "interp":
        return cmd_interp(
            args.file,
            allow_host_side_effects=args.allow_host_side_effects,
            module_paths=_module_search_paths(args.module_path),
        )
    if args.cmd == "repl":
        return run_repl(allow_host_side_effects=args.allow_host_side_effects)
    if args.cmd == "compile":
        return cmd_compile(
            args.file,
            args.output,
            allow_host_side_effects=args.allow_host_side_effects,
            json_output=args.json,
            module_paths=_module_search_paths(args.module_path),
        )
    if args.cmd == "vm":
        return cmd_vm(args.file, allow_host_side_effects=args.allow_host_side_effects)
    if args.cmd == "run":
        return cmd_run(
            args.file,
            allow_host_side_effects=args.allow_host_side_effects,
            module_paths=_module_search_paths(args.module_path),
        )
    if args.cmd == "disasm":
        return cmd_disasm(args.file)
    if args.cmd == "check":
        return cmd_check(
            args.file,
            allow_host_side_effects=args.allow_host_side_effects,
            json_output=args.json,
            module_paths=_module_search_paths(args.module_path),
        )
    if args.cmd == "pkg":
        return run_pkg_cli(args)
    if args.cmd == "host":
        return run_host_cli(args)
    raise AssertionError("unreachable")


def run_pkg_cli(args: argparse.Namespace) -> int:
    if args.pkg_cmd == "init":
        return cmd_pkg_init(
            args.path,
            name=args.name,
            version=args.version,
            main=args.main,
            out_dir=args.out_dir,
            output=args.output,
            allowed_host_calls=args.allowed_host_call,
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


def run_host_cli(args: argparse.Namespace) -> int:
    if args.host_cmd == "list":
        return cmd_host_list(safe_only=args.safe_only, compact=args.compact)
    if args.host_cmd == "describe":
        return cmd_host_describe(safe_only=args.safe_only, compact=args.compact)
    raise AssertionError("unreachable")


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return run_cli(args)
    except AxiomError as e:
        if getattr(args, "json", False):
            payload: dict[str, object] = {
                "ok": False,
                "command": getattr(args, "cmd", None),
                "error": _error_payload(e, args),
            }
            if hasattr(args, "file"):
                payload["file"] = str(args.file)
            if hasattr(args, "output"):
                payload["output"] = str(args.output)
            _emit_json(payload)
        else:
            print(f"error: {e}", file=sys.stderr)
        return 1
