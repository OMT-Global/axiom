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


def _builtin_array_len(args: List[Value], _out: TextIO) -> int:
    arr = args[0]
    if not isinstance(arr, list):
        raise ValueError(f"host.array.len expected an array, got {value_kind(arr)}")
    return len(arr)


def _builtin_array_push_int(args: List[Value], _out: TextIO) -> list:
    arr, val = args
    if not isinstance(arr, list):
        raise ValueError(f"host.array.push_int: expected int[], got {value_kind(arr)}")
    result = list(arr)
    result.append(require_int(val, context="host.array.push_int"))
    return result


def _builtin_array_push_string(args: List[Value], _out: TextIO) -> list:
    arr, val = args
    if not isinstance(arr, list):
        raise ValueError(f"host.array.push_string: expected string[], got {value_kind(arr)}")
    result = list(arr)
    result.append(require_string(val, context="host.array.push_string"))
    return result


def _builtin_array_push_bool(args: List[Value], _out: TextIO) -> list:
    arr, val = args
    if not isinstance(arr, list):
        raise ValueError(f"host.array.push_bool: expected bool[], got {value_kind(arr)}")
    from .values import require_bool
    result = list(arr)
    result.append(require_bool(val, context="host.array.push_bool"))
    return result


def _builtin_array_set_int(args: List[Value], _out: TextIO) -> list:
    arr, idx, val = args
    if not isinstance(arr, list):
        raise ValueError(f"host.array.set_int: expected int[], got {value_kind(arr)}")
    i = require_int(idx, context="host.array.set_int index")
    if i < 0 or i >= len(arr):
        raise ValueError(f"host.array.set_int: index {i} out of bounds (length {len(arr)})")
    result = list(arr)
    result[i] = require_int(val, context="host.array.set_int")
    return result


def _builtin_array_set_string(args: List[Value], _out: TextIO) -> list:
    arr, idx, val = args
    if not isinstance(arr, list):
        raise ValueError(f"host.array.set_string: expected string[], got {value_kind(arr)}")
    i = require_int(idx, context="host.array.set_string index")
    if i < 0 or i >= len(arr):
        raise ValueError(f"host.array.set_string: index {i} out of bounds (length {len(arr)})")
    result = list(arr)
    result[i] = require_string(val, context="host.array.set_string")
    return result


def _builtin_array_set_bool(args: List[Value], _out: TextIO) -> list:
    arr, idx, val = args
    if not isinstance(arr, list):
        raise ValueError(f"host.array.set_bool: expected bool[], got {value_kind(arr)}")
    i = require_int(idx, context="host.array.set_bool index")
    if i < 0 or i >= len(arr):
        raise ValueError(f"host.array.set_bool: index {i} out of bounds (length {len(arr)})")
    from .values import require_bool
    result = list(arr)
    result[i] = require_bool(val, context="host.array.set_bool")
    return result


def _builtin_string_len(args: List[Value], _out: TextIO) -> int:
    return len(require_string(args[0], context="host.string.len"))


def _builtin_string_concat(args: List[Value], _out: TextIO) -> str:
    a = require_string(args[0], context="host.string.concat")
    b = require_string(args[1], context="host.string.concat")
    return a + b


def _builtin_string_contains(args: List[Value], _out: TextIO) -> bool:
    s = require_string(args[0], context="host.string.contains")
    sub = require_string(args[1], context="host.string.contains")
    return sub in s


def _builtin_string_starts_with(args: List[Value], _out: TextIO) -> bool:
    s = require_string(args[0], context="host.string.starts_with")
    prefix = require_string(args[1], context="host.string.starts_with")
    return s.startswith(prefix)


def _builtin_string_ends_with(args: List[Value], _out: TextIO) -> bool:
    s = require_string(args[0], context="host.string.ends_with")
    suffix = require_string(args[1], context="host.string.ends_with")
    return s.endswith(suffix)


def _builtin_string_slice(args: List[Value], _out: TextIO) -> str:
    s = require_string(args[0], context="host.string.slice")
    start = require_int(args[1], context="host.string.slice start")
    end = require_int(args[2], context="host.string.slice end")
    return s[start:end]


def _builtin_string_to_int(args: List[Value], _out: TextIO) -> int:
    value = require_string(args[0], context="host.string.to_int")
    try:
        return int(value.strip())
    except ValueError as e:
        raise ValueError(f"host.string.to_int expected integer string: {value!r}") from e


def _builtin_math_min(args: List[Value], _out: TextIO) -> int:
    a = require_int(args[0], context="host.math.min")
    b = require_int(args[1], context="host.math.min")
    return min(a, b)


def _builtin_math_max(args: List[Value], _out: TextIO) -> int:
    a = require_int(args[0], context="host.math.max")
    b = require_int(args[1], context="host.math.max")
    return max(a, b)


def _builtin_math_pow(args: List[Value], _out: TextIO) -> int:
    base = require_int(args[0], context="host.math.pow")
    exp = require_int(args[1], context="host.math.pow")
    if exp < 0:
        raise ValueError(f"host.math.pow: negative exponent {exp}")
    return base ** exp


HOST_VERSION = VERSION_MINOR

_DEFAULT_HOST_BUILTINS: List[HostBuiltin] = [
    HostBuiltin("version", 0, False, _builtin_version, [], "int"),
    HostBuiltin("print", 1, True, _builtin_print, ["value"], "int"),
    HostBuiltin("read", 1, True, _builtin_read, ["value"], "string"),
    HostBuiltin("abs", 1, False, _builtin_abs, ["int"], "int"),
    HostBuiltin("math.abs", 1, False, _builtin_abs, ["int"], "int"),
    HostBuiltin("int.parse", 1, False, _builtin_parse_int, ["string"], "int"),
    HostBuiltin("array.len", 1, False, _builtin_array_len, ["value"], "int"),
    HostBuiltin("array.push_int", 2, False, _builtin_array_push_int, ["int[]", "int"], "int[]"),
    HostBuiltin("array.push_string", 2, False, _builtin_array_push_string, ["string[]", "string"], "string[]"),
    HostBuiltin("array.push_bool", 2, False, _builtin_array_push_bool, ["bool[]", "bool"], "bool[]"),
    HostBuiltin("array.set_int", 3, False, _builtin_array_set_int, ["int[]", "int", "int"], "int[]"),
    HostBuiltin("array.set_string", 3, False, _builtin_array_set_string, ["string[]", "int", "string"], "string[]"),
    HostBuiltin("array.set_bool", 3, False, _builtin_array_set_bool, ["bool[]", "int", "bool"], "bool[]"),
    HostBuiltin("string.len", 1, False, _builtin_string_len, ["string"], "int"),
    HostBuiltin("string.concat", 2, False, _builtin_string_concat, ["string", "string"], "string"),
    HostBuiltin("string.contains", 2, False, _builtin_string_contains, ["string", "string"], "bool"),
    HostBuiltin("string.starts_with", 2, False, _builtin_string_starts_with, ["string", "string"], "bool"),
    HostBuiltin("string.ends_with", 2, False, _builtin_string_ends_with, ["string", "string"], "bool"),
    HostBuiltin("string.slice", 3, False, _builtin_string_slice, ["string", "int", "int"], "string"),
    HostBuiltin("string.to_int", 1, False, _builtin_string_to_int, ["string"], "int"),
    HostBuiltin("math.min", 2, False, _builtin_math_min, ["int", "int"], "int"),
    HostBuiltin("math.max", 2, False, _builtin_math_max, ["int", "int"], "int"),
    HostBuiltin("math.pow", 2, False, _builtin_math_pow, ["int", "int"], "int"),
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
