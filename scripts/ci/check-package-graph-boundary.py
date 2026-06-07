#!/usr/bin/env python3
"""Validate the compiler.package_graph boundary fixture without Cargo."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "axiom.compiler.package_graph.v1"
CONTRACT = "compiler.package_graph"
DEFAULT_SCHEMA = Path("stage1/compiler-contracts/schemas/axiom.compiler.package_graph.v1.schema.json")
DEFAULT_SNAPSHOT = Path("stage1/compiler-contracts/snapshots/package-graph.json")


def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def load_toml(path: Path) -> dict[str, Any]:
    data: dict[str, Any] = {}
    current: dict[str, Any] = data
    current_array: list[dict[str, Any]] | None = None

    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if not line:
            continue
        if line.startswith("[[") and line.endswith("]]"):
            section = line[2:-2].strip()
            current_array = data.setdefault(section, [])
            if not isinstance(current_array, list):
                fail(f"mixed TOML table types in {path}: {section}")
            current = {}
            current_array.append(current)
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            current_array = None
            current = data.setdefault(section, {})
            if not isinstance(current, dict):
                fail(f"mixed TOML table types in {path}: {section}")
            continue
        if "=" not in line:
            fail(f"unsupported TOML line in {path}: {raw_line}")
        key, value = [part.strip() for part in line.split("=", 1)]
        current[key] = parse_toml_value(value)
    return data


def parse_toml_value(value: str) -> Any:
    value = value.strip()
    if value.startswith('"') and value.endswith('"'):
        return value[1:-1]
    if value in {"true", "false"}:
        return value == "true"
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        if not inner:
            return []
        return [parse_toml_value(part.strip()) for part in inner.split(",")]
    if value.startswith("{") and value.endswith("}"):
        inner = value[1:-1].strip()
        result: dict[str, Any] = {}
        if not inner:
            return result
        for part in inner.split(","):
            key, nested = [item.strip() for item in part.split("=", 1)]
            result[key] = parse_toml_value(nested)
        return result
    if value.isdigit():
        return int(value)
    return value


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def validate_against_schema(value: Any, schema: dict[str, Any]) -> None:
    defs = schema.get("$defs", {})
    validate_schema_node(value, schema, "$", defs)


def validate_schema_node(value: Any, schema: dict[str, Any], path: str, defs: dict[str, Any]) -> None:
    if "$ref" in schema:
        ref = schema["$ref"]
        prefix = "#/$defs/"
        require(ref.startswith(prefix), f"{path} uses unsupported schema ref {ref}")
        name = ref[len(prefix):]
        require(name in defs, f"{path} references unknown schema def {name}")
        validate_schema_node(value, defs[name], path, defs)
        return

    if "const" in schema:
        require(value == schema["const"], f"{path} must equal {schema['const']!r}")

    if "enum" in schema:
        require(value in schema["enum"], f"{path} must be one of {schema['enum']!r}")

    expected_type = schema.get("type")
    if expected_type == "object":
        require(isinstance(value, dict), f"{path} must be an object")
        required = set(schema.get("required", []))
        missing = sorted(required - set(value))
        require(not missing, f"{path} is missing required fields: {', '.join(missing)}")
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            unexpected = sorted(set(value) - set(properties))
            require(not unexpected, f"{path} has unexpected fields: {', '.join(unexpected)}")
        for key, nested in value.items():
            if key in properties:
                validate_schema_node(nested, properties[key], f"{path}.{key}", defs)
    elif expected_type == "array":
        require(isinstance(value, list), f"{path} must be an array")
        if "minItems" in schema:
            require(len(value) >= schema["minItems"], f"{path} must have at least {schema['minItems']} items")
        if schema.get("uniqueItems") is True:
            seen = {json.dumps(item, sort_keys=True) for item in value}
            require(len(seen) == len(value), f"{path} items must be unique")
        item_schema = schema.get("items")
        if item_schema:
            for index, item in enumerate(value):
                validate_schema_node(item, item_schema, f"{path}[{index}]", defs)
    elif expected_type == "string":
        require(isinstance(value, str), f"{path} must be a string")
        if "minLength" in schema:
            require(len(value) >= schema["minLength"], f"{path} must not be empty")
    elif expected_type == "integer":
        require(isinstance(value, int) and not isinstance(value, bool), f"{path} must be an integer")
        if "minimum" in schema:
            require(value >= schema["minimum"], f"{path} must be >= {schema['minimum']}")
    elif expected_type is not None:
        fail(f"{path} uses unsupported schema type {expected_type}")


def reject_cargo_fields(value: Any, path: str = "outputs") -> None:
    if isinstance(value, dict):
        for key, nested in value.items():
            key_lower = key.lower()
            require("cargo" not in key_lower, f"{path}.{key} must not be Cargo-derived")
            require(key not in {"Cargo.toml", "Cargo.lock"}, f"{path}.{key} must not name Cargo files")
            reject_cargo_fields(nested, f"{path}.{key}")
    elif isinstance(value, list):
        for index, nested in enumerate(value):
            reject_cargo_fields(nested, f"{path}[{index}]")
    elif isinstance(value, str):
        text = value.lower()
        require("cargo" not in text, f"{path} must not contain Cargo-derived values")
        require(value not in {"Cargo.toml", "Cargo.lock"}, f"{path} must not name Cargo files")


def package_identity(package: dict[str, Any]) -> dict[str, str]:
    return {
        "name": str(package.get("name", "")),
        "version": str(package.get("version", "")),
        "source": str(package.get("source", "")),
    }


def normalize_dependencies(raw_dependencies: dict[str, Any] | None) -> list[dict[str, str]]:
    normalized = []
    for name, spec in sorted((raw_dependencies or {}).items()):
        if isinstance(spec, str):
            normalized.append({"name": name, "path": spec})
        else:
            entry = {"name": name, "path": str(spec["path"])}
            if "version" in spec:
                entry["version"] = str(spec["version"])
            normalized.append(entry)
    return normalized


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument("--snapshot", type=Path, default=DEFAULT_SNAPSHOT)
    parser.add_argument("--json", action="store_true", help="emit a JSON validation result")
    args = parser.parse_args()

    schema = load_json(args.schema)
    snapshot = load_json(args.snapshot)

    require(schema.get("$id", "").endswith("/axiom.compiler.package_graph.v1.schema.json"), "schema $id must name package graph v1")
    require(schema.get("title") == "Axiom compiler package graph contract", "schema title changed unexpectedly")
    validate_against_schema(snapshot, schema)
    require(snapshot.get("schema_version") == SCHEMA_VERSION, "snapshot schema_version mismatch")
    require(snapshot.get("contract") == CONTRACT, "snapshot contract mismatch")
    reject_cargo_fields(snapshot.get("outputs", {}))

    inputs = snapshot.get("inputs", {})
    outputs = snapshot.get("outputs", {})
    root = Path(inputs.get("root", ""))
    manifest_path = Path(inputs.get("manifest", ""))
    lockfile_path = Path(inputs.get("lockfile", ""))

    for path in [root, manifest_path, lockfile_path]:
        require(path.exists(), f"fixture path does not exist: {path}")

    manifest = load_toml(manifest_path)
    lockfile = load_toml(lockfile_path)
    packages = outputs.get("packages", [])
    lock_packages = lockfile.get("package", [])
    lockfile_integrity = outputs.get("lockfile_integrity", {})

    require(outputs.get("root") == str(root), "output root must match input root")
    require(lockfile.get("version") == 1, "fixture lockfile version must be 1")
    require(lockfile_integrity.get("version") == lockfile.get("version"), "lockfile integrity version mismatch")
    require([package_identity(p) for p in packages] == [package_identity(p) for p in lock_packages], "package graph packages must match axiom.lock identity")
    require(lockfile_integrity.get("packages") == [package_identity(p) for p in lock_packages], "lockfile_integrity packages must match axiom.lock")

    root_package = packages[0]
    manifest_package = manifest.get("package", {})
    require(root_package["name"] == manifest_package.get("name"), "root package name must come from axiom.toml")
    require(root_package["version"] == manifest_package.get("version"), "root package version must come from axiom.toml")
    require(root_package["manifest"] == str(manifest_path), "root package manifest path mismatch")
    require(root_package["lockfile"] == str(lockfile_path), "root package lockfile path mismatch")

    build = manifest.get("build", {})
    require(root_package["entry"] == build.get("entry", "src/main.ax"), "root package entry must come from axiom.toml")
    require(root_package["out_dir"] == build.get("out_dir", "dist"), "root package out_dir must come from axiom.toml")
    require(root_package["workspace_members"] == manifest.get("workspace", {}).get("members", []), "workspace members must come from axiom.toml")
    require(root_package["local_dependencies"] == normalize_dependencies(manifest.get("dependencies")), "local dependencies must come from axiom.toml")

    for package in packages:
        package_manifest = Path(package["manifest"])
        require(package_manifest.exists(), f"package manifest does not exist: {package_manifest}")
        decoded = load_toml(package_manifest)
        declared = decoded.get("package", {})
        require(package["name"] == declared.get("name"), f"{package['manifest']} package name mismatch")
        require(package["version"] == declared.get("version"), f"{package['manifest']} package version mismatch")
        require(Path(package["root"]).exists(), f"package root does not exist: {package['root']}")

    require({"manifest_hash", "lockfile_hash", "source_hashes"}.issubset(set(outputs.get("hash_inputs", []))), "hash inputs must include manifest, lockfile, and sources")
    result = {
        "schema": SCHEMA_VERSION,
        "ok": True,
        "packages": len(packages),
        "fixture": str(args.snapshot),
    }
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"package graph boundary fixture ok: {len(packages)} packages")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
