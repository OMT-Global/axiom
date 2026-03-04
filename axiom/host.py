from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List

from .bytecode import VERSION_MINOR


@dataclass(frozen=True)
class HostBuiltin:
    name: str
    arity: int
    side_effecting: bool


_HOST_BUILTINS_LIST: List[HostBuiltin] = [
    HostBuiltin("version", 0, False),
    HostBuiltin("print", 1, True),
    HostBuiltin("read", 1, True),
    HostBuiltin("abs", 1, False),
    HostBuiltin("math.abs", 1, False),
]

HOST_BUILTINS = {entry.name: (entry.arity, entry.side_effecting) for entry in _HOST_BUILTINS_LIST}
HOST_BUILTIN_NAMES = [entry.name for entry in _HOST_BUILTINS_LIST]
HOST_BUILTIN_IDS = {entry.name: idx for idx, entry in enumerate(_HOST_BUILTINS_LIST)}
HOST_BUILTIN_BY_ID = {idx: entry for idx, entry in enumerate(_HOST_BUILTINS_LIST)}
HOST_VERSION = VERSION_MINOR
