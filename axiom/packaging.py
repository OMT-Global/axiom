from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
import shutil
from typing import Optional

from .api import compile_file
from .errors import AxiomCompileError


MANIFEST_FILENAME = "axiom.pkg"
DEFAULT_NAME = "axiom-app"
DEFAULT_VERSION = "0.1.0"
DEFAULT_MAIN = "src/main.ax"
DEFAULT_OUT_DIR = "dist"


@dataclass(frozen=True)
class PackageManifest:
    name: str
    version: str
    main: str = DEFAULT_MAIN
    out_dir: str = DEFAULT_OUT_DIR
    output: Optional[str] = None


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
    )


def load_manifest(project_root: Path) -> PackageManifest:
    path = manifest_path(project_root)
    if not path.exists():
        raise AxiomCompileError(f"missing package manifest at {path}")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        raise AxiomCompileError(f"invalid package manifest {path}: {e}") from e
    return _as_manifest(payload, path)


def manifest_to_dict(manifest: PackageManifest) -> dict[str, str | None]:
    payload: dict[str, str | None] = {
        "name": manifest.name,
        "version": manifest.version,
        "main": manifest.main,
        "out_dir": manifest.out_dir,
    }
    if manifest.output is not None:
        payload["output"] = manifest.output
    return payload


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
    pkg_name = name if name else (project_root.name or DEFAULT_NAME)
    if not pkg_name:
        pkg_name = DEFAULT_NAME
    manifest = PackageManifest(
        name=pkg_name,
        version=version,
        main=main,
        out_dir=out_dir,
        output=output,
    )
    write_default_manifest(project_root, manifest)
    write_default_entry(project_root, manifest.main)
    return manifest


def build_package(project_root: Path, *, allow_host_side_effects: bool = False) -> Path:
    project_root = project_root.resolve()
    manifest = load_manifest(project_root)
    entry = project_root / manifest.main
    bytecode = compile_file(entry, allow_host_side_effects=allow_host_side_effects)
    out_dir = project_root / manifest.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    output = manifest.output or manifest.name
    if output.endswith(".axb"):
        output_name = output
    else:
        output_name = f"{output}.axb"

    out_path = out_dir / output_name
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(bytecode.encode())
    return out_path


def clean_package(project_root: Path) -> bool:
    project_root = project_root.resolve()
    manifest = load_manifest(project_root)
    out_dir = project_root / manifest.out_dir
    if not out_dir.exists():
        return False
    if out_dir.is_file():
        raise AxiomCompileError(f"package out_dir is not a directory: {out_dir}")
    shutil.rmtree(out_dir)
    return True
