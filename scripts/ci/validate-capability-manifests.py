#!/usr/bin/env python3
"""Validate Axiom capability manifest tables with CI-cheap checks."""

from __future__ import annotations

import argparse
import ast
import sys
from pathlib import Path


BOOL_KEYS = {
    "fs",
    "fs:write",
    "net",
    "process",
    "env_unrestricted",
    "clock",
    "crypto",
    "ffi",
}
KNOWN_KEYS = BOOL_KEYS | {"fs_root", "env"}


def iter_manifests(root: Path) -> list[Path]:
    return sorted(
        path
        for path in root.rglob("axiom.toml")
        if ".axiom-build" not in path.parts and ".git" not in path.parts
    )


def validate_manifest(path: Path) -> list[str]:
    errors: list[str] = []
    try:
        capabilities = read_capabilities_table(path)
    except (OSError, ValueError) as exc:
        return [f"{path}: failed to parse capability table: {exc}"]

    if capabilities is None:
        return errors

    for key, value in capabilities.items():
        if key not in KNOWN_KEYS:
            errors.append(f"{path}: unknown [capabilities] key {key!r}")
            continue
        if key in BOOL_KEYS and not isinstance(value, bool):
            errors.append(f"{path}: [capabilities].{key} must be a boolean")
        elif key == "fs_root":
            validate_fs_root(path, value, errors)
        elif key == "env":
            validate_env(path, value, errors)
    return errors


def read_capabilities_table(path: Path) -> dict[str, object] | None:
    in_capabilities = False
    found = False
    values: dict[str, object] = {}
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = strip_comment(raw_line).strip()
        if not line:
            continue
        if line.startswith("[") and line.endswith("]"):
            table = line.strip("[]").strip()
            in_capabilities = table == "capabilities"
            found = found or in_capabilities
            continue
        if not in_capabilities:
            continue
        if "=" not in line:
            raise ValueError(f"{line_number}: expected key = value")
        key, raw_value = line.split("=", 1)
        key = parse_key(key.strip(), line_number)
        if key in values:
            raise ValueError(f"{line_number}: duplicate [capabilities] key {key!r}")
        values[key] = parse_value(raw_value.strip(), line_number)
    return values if found else None


def strip_comment(line: str) -> str:
    in_string = False
    escaped = False
    for index, char in enumerate(line):
        if char == "\\" and in_string:
            escaped = not escaped
            continue
        if char == '"' and not escaped:
            in_string = not in_string
        if char == "#" and not in_string:
            return line[:index]
        escaped = False
    return line


def parse_value(raw_value: str, line_number: int) -> object:
    if raw_value == "true":
        return True
    if raw_value == "false":
        return False
    if raw_value.startswith('"') or raw_value.startswith("["):
        try:
            return ast.literal_eval(raw_value)
        except (SyntaxError, ValueError) as exc:
            raise ValueError(f"{line_number}: invalid capability value {raw_value!r}: {exc}") from exc
    raise ValueError(f"{line_number}: unsupported capability value {raw_value!r}")


def parse_key(raw_key: str, line_number: int) -> str:
    if raw_key.startswith('"'):
        try:
            key = ast.literal_eval(raw_key)
        except (SyntaxError, ValueError) as exc:
            raise ValueError(f"{line_number}: invalid capability key {raw_key!r}: {exc}") from exc
        if not isinstance(key, str):
            raise ValueError(f"{line_number}: capability key must be a string")
        return key
    return raw_key


def validate_fs_root(path: Path, value: object, errors: list[str]) -> None:
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{path}: [capabilities].fs_root must be a non-empty string")
        return
    candidate = Path(value)
    if candidate.is_absolute():
        errors.append(f"{path}: [capabilities].fs_root must be relative")
    if ".." in candidate.parts:
        errors.append(f"{path}: [capabilities].fs_root must not use parent traversal")


def validate_env(path: Path, value: object, errors: list[str]) -> None:
    if isinstance(value, bool):
        return
    if not isinstance(value, list):
        errors.append(f"{path}: [capabilities].env must be a boolean or string list")
        return

    seen: set[str] = set()
    for index, item in enumerate(value):
        field = f"[capabilities].env[{index}]"
        if not isinstance(item, str) or not item.strip():
            errors.append(f"{path}: {field} must be a non-empty string")
            continue
        if "=" in item:
            errors.append(f"{path}: {field} must be a variable name, not NAME=value")
        if item in seen:
            errors.append(f"{path}: duplicate environment allowlist entry at {field}")
        seen.add(item)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()

    root = args.root.resolve()
    errors: list[str] = []
    for manifest in iter_manifests(root):
        errors.extend(validate_manifest(manifest))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"validated capability manifests under {root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
