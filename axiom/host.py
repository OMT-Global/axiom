from __future__ import annotations

import builtins as py_builtins
import hashlib
import json
from dataclasses import dataclass
from typing import Callable, Dict, List, TextIO

from .bytecode import VERSION_MINOR
from .values import (
    Value,
    ValueKind,
    kind_matches,
    render_value,
    require_int,
    require_string,
    value_kind,
)


Handler = Callable[[List[Value], TextIO], Value]


@dataclass(frozen=True)
class HostBuiltin:
    name: str
    arity: int
    side_effecting: bool
    handler: Handler
    arg_kinds: List[ValueKind]
    return_kind: ValueKind


def _builtin_version(_args: List[Value], _out: TextIO) -> int:
    return HOST_VERSION


def _builtin_print(args: List[Value], out: TextIO) -> int:
    out.write(f"{render_value(args[0])}\n")
    return 0


def _builtin_read(args: List[Value], _out: TextIO) -> str:
    prompt = render_value(args[0])
    try:
        line = py_builtins.input(prompt)
    except EOFError:
        return ""
    return line


def _builtin_abs(args: List[Value], _out: TextIO) -> int:
    return abs(require_int(args[0], context="host.abs"))


def _builtin_parse_int(args: List[Value], _out: TextIO) -> int:
    value = require_string(args[0], context="host.int.parse")
    try:
        return int(value.strip())
    except ValueError as e:
        raise ValueError(f"host.int.parse expected integer string: {value!r}") from e


HOST_VERSION = VERSION_MINOR

_DEFAULT_HOST_BUILTINS: List[HostBuiltin] = [
    HostBuiltin("version", 0, False, _builtin_version, [], "int"),
    HostBuiltin("print", 1, True, _builtin_print, ["value"], "int"),
    HostBuiltin("read", 1, True, _builtin_read, ["value"], "string"),
    HostBuiltin("abs", 1, False, _builtin_abs, ["int"], "int"),
    HostBuiltin("math.abs", 1, False, _builtin_abs, ["int"], "int"),
    HostBuiltin("int.parse", 1, False, _builtin_parse_int, ["string"], "int"),
]

_HOST_BUILTINS_LIST: List[HostBuiltin] = list(_DEFAULT_HOST_BUILTINS)

HOST_BUILTINS: Dict[str, HostBuiltin] = {}
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
        HOST_BUILTINS[entry.name] = entry
        HOST_BUILTIN_NAMES.append(entry.name)
        HOST_BUILTIN_IDS[entry.name] = idx
        HOST_BUILTIN_BY_ID[idx] = entry


def register_host_builtin(
    name: str,
    arity: int,
    side_effecting: bool,
    handler: Handler,
    *,
    arg_kinds: List[ValueKind] | None = None,
    return_kind: ValueKind = "int",
) -> None:
    if name in HOST_BUILTINS:
        raise ValueError(f"host builtin {name!r} already exists")
    if arity < 0:
        raise ValueError(f"host builtin arity must be non-negative, got {arity}")
    normalized_arg_kinds = list(arg_kinds) if arg_kinds is not None else ["int"] * arity
    if len(normalized_arg_kinds) != arity:
        raise ValueError(
            f"host builtin {name!r} declared arity {arity} but got {len(normalized_arg_kinds)} arg kinds"
        )
    for kind in normalized_arg_kinds:
        if kind not in {"int", "string", "bool", "value"}:
            raise ValueError(f"host builtin {name!r} has invalid arg kind {kind!r}")
    if return_kind not in {"int", "string", "bool", "value"}:
        raise ValueError(f"host builtin {name!r} has invalid return kind {return_kind!r}")
    _HOST_BUILTINS_LIST.append(
        HostBuiltin(
            name=name,
            arity=arity,
            side_effecting=side_effecting,
            handler=handler,
            arg_kinds=normalized_arg_kinds,
            return_kind=return_kind,
        )
    )
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


def call_host_builtin(name: str, args: List[Value], out: TextIO) -> Value:
    if name not in HOST_BUILTIN_IDS:
        raise KeyError(f"undefined host function {name!r}")
    return call_host_builtin_id(HOST_BUILTIN_IDS[name], args, out)


def call_host_builtin_id(host_fn_id: int, args: List[Value], out: TextIO) -> Value:
    builtin = HOST_BUILTIN_BY_ID[host_fn_id]
    if len(args) != builtin.arity:
        raise ValueError(
            f"host function {builtin.name!r} expects {builtin.arity} args, got {len(args)}"
        )
    for index, (value, expected_kind) in enumerate(zip(args, builtin.arg_kinds, strict=True)):
        if not kind_matches(value, expected_kind):
            raise ValueError(
                f"host function {builtin.name!r} argument {index + 1} expects {expected_kind}, got {value_kind(value)}"
            )
    result = builtin.handler(args, out)
    if not kind_matches(result, builtin.return_kind):
        raise ValueError(
            f"host function {builtin.name!r} returned {value_kind(result)}, expected {builtin.return_kind}"
        )
    return result


def host_capabilities(safe_only: bool = False) -> List[Dict[str, object]]:
    payload: List[Dict[str, object]] = []
    for name in sorted(HOST_BUILTIN_NAMES):
        builtin = HOST_BUILTINS[name]
        if safe_only and builtin.side_effecting:
            continue
        payload.append(
            {
                "name": name,
                "arity": builtin.arity,
                "side_effecting": builtin.side_effecting,
                "arg_kinds": list(builtin.arg_kinds),
                "return_kind": builtin.return_kind,
            }
        )
    return payload


def host_contract_metadata(safe_only: bool = False) -> Dict[str, object]:
    caps = host_capabilities(safe_only=safe_only)
    signature = hashlib.sha256(
        json.dumps(caps, sort_keys=True).encode("utf-8")
    ).hexdigest()
    return {
        "schema_version": 1,
        "runtime_version_minor": VERSION_MINOR,
        "capabilities": caps,
        "capabilities_signature": signature,
    }


_rebuild_host_tables()
