from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
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
    if not isinstance(main, str) or not main:
        raise AxiomCompileError(f"package manifest {path} has invalid main")
    if not isinstance(out_dir, str) or not out_dir:
        raise AxiomCompileError(f"package manifest {path} has invalid out_dir")
    if output is not None and (not isinstance(output, str) or not output):
        raise AxiomCompileError(f"package manifest {path} has invalid output")
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


def default_manifest(name: str) -> PackageManifest:
    return PackageManifest(name=name, version=DEFAULT_VERSION)


def write_default_manifest(project_root: Path, manifest: PackageManifest) -> PackageManifest:
    path = manifest_path(project_root)
    payload = {
        "name": manifest.name,
        "version": manifest.version,
        "main": manifest.main,
        "out_dir": manifest.out_dir,
    }
    if manifest.output is not None:
        payload["output"] = manifest.output
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    return manifest


def write_default_entry(project_root: Path) -> Path:
    path = project_root / DEFAULT_MAIN
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        path.write_text("print 0\n", encoding="utf-8")
    return path


def init_package(project_root: Path, *, name: Optional[str] = None) -> PackageManifest:
    project_root = project_root.resolve()
    project_root.mkdir(parents=True, exist_ok=True)
    if manifest_path(project_root).exists():
        raise AxiomCompileError(
            f"package manifest already exists at {manifest_path(project_root)}"
        )
    pkg_name = name if name else (project_root.name or DEFAULT_NAME)
    manifest = default_manifest(pkg_name)
    write_default_manifest(project_root, manifest)
    write_default_entry(project_root)
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
    out_path.write_bytes(bytecode.encode())
    return out_path
