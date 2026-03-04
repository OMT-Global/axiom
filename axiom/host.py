from __future__ import annotations

import builtins as py_builtins
from dataclasses import dataclass
from typing import Callable, Dict, List, TextIO, Tuple

from .bytecode import VERSION_MINOR


Handler = Callable[[List[int], TextIO], int]


@dataclass(frozen=True)
class HostBuiltin:
    name: str
    arity: int
    side_effecting: bool
    handler: Handler


def _builtin_version(_args: List[int], _out: TextIO) -> int:
    return HOST_VERSION


def _builtin_print(args: List[int], out: TextIO) -> int:
    out.write(f"{args[0]}\n")
    return 0


def _builtin_read(args: List[int], _out: TextIO) -> int:
    prompt = str(args[0])
    try:
        line = py_builtins.input(prompt)
    except EOFError:
        return 0
    try:
        return int(line.strip())
    except ValueError as e:
        raise ValueError(f"host.read expected integer input: {line!r}") from e


def _builtin_abs(args: List[int], _out: TextIO) -> int:
    return abs(args[0])


HOST_VERSION = VERSION_MINOR

_DEFAULT_HOST_BUILTINS: List[HostBuiltin] = [
    HostBuiltin("version", 0, False, _builtin_version),
    HostBuiltin("print", 1, True, _builtin_print),
    HostBuiltin("read", 1, True, _builtin_read),
    HostBuiltin("abs", 1, False, _builtin_abs),
    HostBuiltin("math.abs", 1, False, _builtin_abs),
]

_HOST_BUILTINS_LIST: List[HostBuiltin] = list(_DEFAULT_HOST_BUILTINS)

HOST_BUILTINS: Dict[str, Tuple[int, bool]] = {}
HOST_BUILTIN_NAMES: List[str] = []
HOST_BUILTIN_IDS: Dict[str, int] = {}
HOST_BUILTIN_BY_ID: Dict[int, HostBuiltin] = {}


def _rebuild_host_tables() -> None:
    HOST_BUILTINS.clear()
    HOST_BUILTIN_NAMES.clear()
    HOST_BUILTIN_IDS.clear()
    HOST_BUILTIN_BY_ID.clear()
    for idx, entry in enumerate(_HOST_BUILTINS_LIST):
        if entry.name in HOST_BUILTIN_IDS:
            raise ValueError(f"duplicate host builtin name {entry.name!r}")
        HOST_BUILTINS[entry.name] = (entry.arity, entry.side_effecting)
        HOST_BUILTIN_NAMES.append(entry.name)
        HOST_BUILTIN_IDS[entry.name] = idx
        HOST_BUILTIN_BY_ID[idx] = entry


def register_host_builtin(name: str, arity: int, side_effecting: bool, handler: Handler) -> None:
    if name in HOST_BUILTINS:
        raise ValueError(f"host builtin {name!r} already exists")
    if arity < 0:
        raise ValueError(f"host builtin arity must be non-negative, got {arity}")
    _HOST_BUILTINS_LIST.append(HostBuiltin(name=name, arity=arity, side_effecting=side_effecting, handler=handler))
    _rebuild_host_tables()


def unregister_host_builtin(name: str) -> None:
    if any(entry.name == name for entry in _DEFAULT_HOST_BUILTINS):
        raise ValueError(f"cannot unregister builtin {name!r}")

    for idx, entry in enumerate(_HOST_BUILTINS_LIST):
        if entry.name == name:
            del _HOST_BUILTINS_LIST[idx]
            _rebuild_host_tables()
            return

    raise KeyError(f"host builtin {name!r} does not exist")


def reset_host_builtins() -> None:
    del _HOST_BUILTINS_LIST[:]
    _HOST_BUILTINS_LIST.extend(_DEFAULT_HOST_BUILTINS)
    _rebuild_host_tables()


def call_host_builtin(name: str, args: List[int], out: TextIO) -> int:
    if name not in HOST_BUILTIN_IDS:
        raise KeyError(f"undefined host function {name!r}")
    return call_host_builtin_id(HOST_BUILTIN_IDS[name], args, out)


def call_host_builtin_id(host_fn_id: int, args: List[int], out: TextIO) -> int:
    return HOST_BUILTIN_BY_ID[host_fn_id].handler(args, out)


def host_capabilities(safe_only: bool = False) -> List[Dict[str, object]]:
    payload: List[Dict[str, object]] = []
    for name in sorted(HOST_BUILTIN_NAMES):
        arity, side_effecting = HOST_BUILTINS[name]
        if safe_only and side_effecting:
            continue
        payload.append(
            {
                "name": name,
                "arity": arity,
                "side_effecting": side_effecting,
            }
        )
    return payload


def host_contract_metadata(safe_only: bool = False) -> Dict[str, object]:
    return {
        "schema_version": 1,
        "runtime_version_minor": VERSION_MINOR,
        "capabilities": host_capabilities(safe_only=safe_only),
    }


_rebuild_host_tables()
