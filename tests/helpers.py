from __future__ import annotations

import io
import json
import subprocess
import sys
from pathlib import Path
from typing import Any
from unittest import TestCase

from axiom.api import compile_to_bytecode, parse_program
from axiom.interpreter import Interpreter
from axiom.vm import Vm


ROOT = Path(__file__).resolve().parents[1]
PROGRAMS_DIR = ROOT / "tests" / "programs"


def run_cli(
    testcase: TestCase,
    args: list[str],
    *,
    cwd: Path = ROOT,
    expect_code: int = 0,
    input_text: str | None = None,
) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        [sys.executable, "-m", "axiom", *args],
        capture_output=True,
        text=True,
        cwd=str(cwd),
        input=input_text,
    )
    testcase.assertEqual(
        proc.returncode,
        expect_code,
        msg=f"{' '.join(args)} failed (expected {expect_code}): {proc.stdout}\n{proc.stderr}",
    )
    return proc


def run_cli_json(
    testcase: TestCase,
    args: list[str],
    *,
    cwd: Path = ROOT,
    expect_code: int = 0,
) -> dict[str, Any]:
    proc = run_cli(testcase, args, cwd=cwd, expect_code=expect_code)
    return json.loads(proc.stdout)


def load_program_fixture(path: Path) -> tuple[str, str]:
    return (
        path.read_text(encoding="utf-8"),
        path.with_suffix(".out").read_text(encoding="utf-8"),
    )


def run_interpreter(src: str) -> str:
    program = parse_program(src)
    out = io.StringIO()
    Interpreter().run(program, out)
    return out.getvalue()


def run_vm(src: str) -> str:
    bytecode = compile_to_bytecode(src)
    out = io.StringIO()
    Vm(locals_count=bytecode.locals_count).run(bytecode, out)
    return out.getvalue()


def assert_program_parity(
    testcase: TestCase,
    src: str,
    expected: str,
    *,
    label: str,
) -> None:
    interp_out = run_interpreter(src)
    vm_out = run_vm(src)
    testcase.assertEqual(interp_out, expected, f"interpreter mismatch: {label}")
    testcase.assertEqual(vm_out, expected, f"vm mismatch: {label}")


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(json.dumps(payload), encoding="utf-8")


def init_temp_package(
    testcase: TestCase,
    project: Path,
    *,
    name: str = "demo",
    extra_args: list[str] | None = None,
) -> dict[str, Any]:
    args = ["pkg", "init", str(project), "--name", name]
    if extra_args:
        args.extend(extra_args)
    run_cli(testcase, args)
    return read_json(project / "axiom.pkg")
