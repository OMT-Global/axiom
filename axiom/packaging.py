from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
import shutil
from typing import List, Optional, Set, TextIO

from .api import compile_file
from .bytecode import Bytecode
from .host import host_contract_metadata
from .errors import AxiomCompileError
from .vm import Vm


MANIFEST_FILENAME = "axiom.pkg"
DEFAULT_NAME = "axiom-app"
DEFAULT_VERSION = "0.1.0"
DEFAULT_MAIN = "src/main.ax"
DEFAULT_OUT_DIR = "dist"
MAX_MANIFEST_BYTES = 1 * 1024 * 1024


@dataclass(frozen=True)
class PackageManifest:
    name: str
    version: str
    main: str = DEFAULT_MAIN
    out_dir: str = DEFAULT_OUT_DIR
    output: Optional[str] = None
    allowed_host_calls: Optional[List[str]] = None
    host_contract_signature: Optional[str] = None


@dataclass(frozen=True)
class PreparedPackage:
    project_root: Path
    manifest: PackageManifest
    entry: Path
    allowed_host_calls: Optional[Set[str]]


def manifest_path(project_root: Path) -> Path:
    return project_root / MANIFEST_FILENAME


def _validate_output(output: str, path: Path) -> str:
    if not output:
        raise AxiomCompileError(f"package manifest {path} has invalid output")

    if Path(output).is_absolute():
        raise AxiomCompileError(
            f"package manifest {path} has invalid output path: absolute paths are not allowed"
        )

    if any(part == ".." for part in Path(output).parts):
        raise AxiomCompileError(
            f"package manifest {path} has invalid output path: parent traversal is not allowed"
        )

    return output


def _validate_relative_path(value: str, path: Path, field_name: str) -> str:
    if not value:
        raise AxiomCompileError(f"package manifest {path} has invalid {field_name}")

    candidate = Path(value)
    if candidate.is_absolute():
        raise AxiomCompileError(
            f"package manifest {path} has invalid {field_name}: absolute paths are not allowed"
        )

    if any(part == ".." for part in candidate.parts):
        raise AxiomCompileError(
            f"package manifest {path} has invalid {field_name}: parent traversal is not allowed"
        )

    return value


def _validate_host_calls(value: object, path: Path) -> Optional[List[str]]:
    if value is None:
        return None
    if not isinstance(value, list):
        raise AxiomCompileError(f"package manifest {path} has invalid allowed_host_calls")
    host_calls: List[str] = []
    for item in value:
        if not isinstance(item, str) or not item:
            raise AxiomCompileError(
                f"package manifest {path} has invalid allowed_host_calls entry {item!r}"
            )
        normalized = item[5:] if item.startswith("host.") else item
        if not normalized:
            raise AxiomCompileError(
                f"package manifest {path} has invalid allowed_host_calls entry {item!r}"
            )
        host_calls.append(normalized)
    return host_calls


def _validate_host_contract_signature(value: object, path: Path) -> Optional[str]:
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise AxiomCompileError(
            f"package manifest {path} has invalid host_contract_signature"
        )
    lowered = value.lower()
    if len(lowered) != 64 or any(c not in "0123456789abcdef" for c in lowered):
        raise AxiomCompileError(
            f"package manifest {path} has invalid host_contract_signature"
        )
    return lowered


def _as_manifest(data: dict[str, object], path: Path) -> PackageManifest:
    if not isinstance(data, dict):
        raise AxiomCompileError(f"invalid package manifest in {path}")
    if not isinstance(data.get("name"), str) or not data["name"]:
        raise AxiomCompileError(f"package manifest {path} missing name")
    if not isinstance(data.get("version"), str) or not data["version"]:
        raise AxiomCompileError(f"package manifest {path} missing version")
    main = data.get("main", DEFAULT_MAIN)
    out_dir = data.get("out_dir", DEFAULT_OUT_DIR)
    output = data.get("output")
    if not isinstance(main, str):
        raise AxiomCompileError(f"package manifest {path} has invalid main")
    main = _validate_relative_path(main, path, "main")
    if not isinstance(out_dir, str):
        raise AxiomCompileError(f"package manifest {path} has invalid out_dir")
    out_dir = _validate_relative_path(out_dir, path, "out_dir")
    allowed_host_calls = _validate_host_calls(data.get("allowed_host_calls"), path)
    host_contract_signature = _validate_host_contract_signature(
        data.get("host_contract_signature"), path
    )
    if output is not None:
        if not isinstance(output, str):
            raise AxiomCompileError(f"package manifest {path} has invalid output")
        output = _validate_output(output, path)

    return PackageManifest(
        name=str(data["name"]),
        version=str(data["version"]),
        main=main,
        out_dir=out_dir,
        output=output if isinstance(output, str) else None,
        allowed_host_calls=allowed_host_calls,
        host_contract_signature=host_contract_signature,
    )


def _validate_project_host_contract(
    manifest: PackageManifest, path: Path
) -> None:
    if manifest.host_contract_signature is None:
        return
    runtime_signature = str(host_contract_metadata()["capabilities_signature"])
    if manifest.host_contract_signature != runtime_signature:
        raise AxiomCompileError(
            f"package manifest {path} has host_contract_signature mismatch: "
            f"expected {manifest.host_contract_signature}, runtime {runtime_signature}"
        )


def load_manifest(project_root: Path) -> PackageManifest:
    path = manifest_path(project_root)
    if not path.exists():
        raise AxiomCompileError(f"missing package manifest at {path}")
    manifest_size = path.stat().st_size
    if manifest_size > MAX_MANIFEST_BYTES:
        raise AxiomCompileError(
            f"package manifest {path} is too large "
            f"({manifest_size} bytes, max {MAX_MANIFEST_BYTES})"
        )
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        raise AxiomCompileError(f"invalid package manifest {path}: {e}") from e
    return _as_manifest(payload, path)


def manifest_to_dict(manifest: PackageManifest) -> dict[str, object]:
    payload: dict[str, object] = {
        "name": manifest.name,
        "version": manifest.version,
        "main": manifest.main,
        "out_dir": manifest.out_dir,
    }
    if manifest.output is not None:
        payload["output"] = manifest.output
    if manifest.allowed_host_calls is not None:
        payload["allowed_host_calls"] = manifest.allowed_host_calls
    if manifest.host_contract_signature is not None:
        payload["host_contract_signature"] = manifest.host_contract_signature
    return payload


def prepare_package(
    project_root: Path, *, validate_host_contract: bool = True
) -> PreparedPackage:
    project_root = project_root.resolve()
    manifest = load_manifest(project_root)
    if validate_host_contract:
        _validate_project_host_contract(manifest, manifest_path(project_root))
    return PreparedPackage(
        project_root=project_root,
        manifest=manifest,
        entry=project_root / manifest.main,
        allowed_host_calls=(
            set(manifest.allowed_host_calls)
            if manifest.allowed_host_calls is not None
            else None
        ),
    )


def write_default_manifest(project_root: Path, manifest: PackageManifest) -> PackageManifest:
    path = manifest_path(project_root)
    payload = manifest_to_dict(manifest)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    return manifest


def write_default_entry(project_root: Path, main: str) -> Path:
    path = project_root / main
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        path.write_text("print 0\n", encoding="utf-8")
    return path


def init_package(
    project_root: Path,
    *,
    name: Optional[str] = None,
    version: Optional[str] = None,
    main: Optional[str] = None,
    out_dir: Optional[str] = None,
    output: Optional[str] = None,
    host_contract_signature: Optional[str] = None,
    allowed_host_calls: Optional[List[str]] = None,
    force: bool = False,
) -> PackageManifest:
    project_root = project_root.resolve()
    project_root.mkdir(parents=True, exist_ok=True)
    if manifest_path(project_root).exists() and not force:
        raise AxiomCompileError(
            f"package manifest already exists at {manifest_path(project_root)}"
        )
    if name is not None and (not isinstance(name, str) or not name):
        raise AxiomCompileError("package name must be a non-empty string")
    version = version if version is not None else DEFAULT_VERSION
    main = main if main is not None else DEFAULT_MAIN
    out_dir = out_dir if out_dir is not None else DEFAULT_OUT_DIR
    main = _validate_relative_path(main, project_root / MANIFEST_FILENAME, "main")
    out_dir = _validate_relative_path(out_dir, project_root / MANIFEST_FILENAME, "out_dir")
    if not isinstance(version, str) or not version:
        raise AxiomCompileError("package version must be a non-empty string")
    if not isinstance(main, str) or not main:
        raise AxiomCompileError("package main must be a non-empty string")
    if not isinstance(out_dir, str) or not out_dir:
        raise AxiomCompileError("package out_dir must be a non-empty string")
    if output is not None and not isinstance(output, str):
        raise AxiomCompileError("package output must be a non-empty string when provided")
    if output is not None:
        output = _validate_output(output, project_root / MANIFEST_FILENAME)
    if host_contract_signature is None:
        host_contract_signature = str(host_contract_metadata()["capabilities_signature"])
    host_contract_signature = _validate_host_contract_signature(
        host_contract_signature, project_root / MANIFEST_FILENAME
    )
    if allowed_host_calls is None:
        allowed_host_calls = None
    elif not isinstance(allowed_host_calls, list):
        raise AxiomCompileError("package allowed_host_calls must be a list of strings when provided")
    allowed_host_calls = _validate_host_calls(allowed_host_calls, project_root / MANIFEST_FILENAME)
    pkg_name = name if name else (project_root.name or DEFAULT_NAME)
    if not pkg_name:
        pkg_name = DEFAULT_NAME
    manifest = PackageManifest(
        name=pkg_name,
        version=version,
        main=main,
        out_dir=out_dir,
        output=output,
        host_contract_signature=host_contract_signature,
        allowed_host_calls=allowed_host_calls,
    )
    write_default_manifest(project_root, manifest)
    write_default_entry(project_root, manifest.main)
    return manifest


def build_package(
    project_root: Path,
    *,
    allow_host_side_effects: bool = False,
    output: Optional[str] = None,
) -> Path:
    prepared, bytecode = compile_package(
        project_root,
        allow_host_side_effects=allow_host_side_effects,
    )
    out_dir = prepared.project_root / prepared.manifest.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    selected_output = (
        output
        if output is not None
        else prepared.manifest.output or prepared.manifest.name
    )
    validated_output = _validate_output(
        selected_output, manifest_path(prepared.project_root)
    )

    if validated_output.endswith(".axb"):
        output_name = validated_output
    else:
        output_name = f"{validated_output}.axb"

    out_path = out_dir / output_name
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(bytecode.encode())
    return out_path


def check_package(
    project_root: Path,
    *,
    allow_host_side_effects: bool = False,
) -> PreparedPackage:
    prepared, _ = compile_package(
        project_root,
        allow_host_side_effects=allow_host_side_effects,
    )
    return prepared


def compile_package(
    project_root: Path,
    *,
    allow_host_side_effects: bool = False,
) -> tuple[PreparedPackage, Bytecode]:
    prepared = prepare_package(project_root)
    bytecode = compile_file(
        prepared.entry,
        allow_host_side_effects=allow_host_side_effects,
        allowed_host_calls=prepared.allowed_host_calls,
    )
    return prepared, bytecode


def run_package(
    project_root: Path,
    *,
    allow_host_side_effects: bool = False,
    out: TextIO,
) -> PreparedPackage:
    prepared, bytecode = compile_package(
        project_root,
        allow_host_side_effects=allow_host_side_effects,
    )
    Vm(
        locals_count=bytecode.locals_count,
        allow_host_side_effects=allow_host_side_effects,
    ).run(bytecode, out)
    return prepared


def clean_package(project_root: Path) -> bool:
    prepared = prepare_package(project_root, validate_host_contract=False)
    out_dir = prepared.project_root / prepared.manifest.out_dir
    if not out_dir.exists():
        return False
    if out_dir.is_file():
        raise AxiomCompileError(f"package out_dir is not a directory: {out_dir}")
    shutil.rmtree(out_dir)
    return True
