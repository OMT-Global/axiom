from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from .intops import trunc_div


@dataclass(frozen=True)
class FunctionValue:
    """A first-class function reference, holding the canonical function name."""

    name: str


Value = int | str | bool | list | FunctionValue
ValueKind = Literal["int", "string", "bool", "int[]", "string[]", "bool[]", "fn", "value"]


def value_kind(value: Value) -> ValueKind:
    if type(value) is bool:
        return "bool"
    if type(value) is int:
        return "int"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        # Infer element type from the first element when possible.
        if value:
            first = value[0]
            if type(first) is bool:
                return "bool[]"
            if type(first) is int:
                return "int[]"
            if isinstance(first, str):
                return "string[]"
        return "value"  # empty array — type not inferrable at runtime
    if isinstance(value, FunctionValue):
        return "fn"
    return "value"


def kind_matches(value: Value, expected: ValueKind) -> bool:
    if expected == "value":
        return True
    vk = value_kind(value)
    # A typed array is also accepted by the generic "value" kind (handled above).
    # An array with no elements matches any array kind.
    if isinstance(value, list) and not value and expected in ("int[]", "string[]", "bool[]"):
        return True
    return vk == expected


def render_value(value: Value) -> str:
    if isinstance(value, FunctionValue):
        return f"<fn {value.name}>"
    if isinstance(value, list):
        return "[" + ", ".join(render_value(v) for v in value) + "]"
    if isinstance(value, str):
        return value
    if type(value) is bool:
        return "true" if value else "false"
    return str(value)


def require_int(value: Value, *, context: str) -> int:
    if type(value) is int:
        return value
    raise ValueError(f"{context} expected int, got {value_kind(value)}")


def require_string(value: Value, *, context: str) -> str:
    if isinstance(value, str):
        return value
    raise ValueError(f"{context} expected string, got {value_kind(value)}")


def require_bool(value: Value, *, context: str) -> bool:
    if type(value) is bool:
        return value
    raise ValueError(f"{context} expected bool, got {value_kind(value)}")


def add_values(lhs: Value, rhs: Value, *, context: str) -> Value:
    if type(lhs) is int and type(rhs) is int:
        return lhs + rhs
    if isinstance(lhs, str) and isinstance(rhs, str):
        return lhs + rhs
    raise ValueError(
        f"{context} expected matching int or string operands, got {value_kind(lhs)} and {value_kind(rhs)}"
    )


def sub_values(lhs: Value, rhs: Value, *, context: str) -> int:
    return require_int(lhs, context=context) - require_int(rhs, context=context)


def mul_values(lhs: Value, rhs: Value, *, context: str) -> int:
    return require_int(lhs, context=context) * require_int(rhs, context=context)


def div_values(lhs: Value, rhs: Value, *, context: str) -> int:
    left = require_int(lhs, context=context)
    right = require_int(rhs, context=context)
    if right == 0:
        raise ValueError("division by zero")
    return trunc_div(left, right)


def negate_value(value: Value, *, context: str) -> int:
    return -require_int(value, context=context)


def compare_eq(lhs: Value, rhs: Value) -> bool:
    if value_kind(lhs) != value_kind(rhs):
        raise ValueError(
            f"operator '==' expects matching operand types, got {value_kind(lhs)} and {value_kind(rhs)}"
        )
    return lhs == rhs


def compare_ne(lhs: Value, rhs: Value) -> bool:
    if value_kind(lhs) != value_kind(rhs):
        raise ValueError(
            f"operator '!=' expects matching operand types, got {value_kind(lhs)} and {value_kind(rhs)}"
        )
    return lhs != rhs


def compare_lt(lhs: Value, rhs: Value, *, context: str) -> bool:
    return require_int(lhs, context=context) < require_int(rhs, context=context)


def compare_le(lhs: Value, rhs: Value, *, context: str) -> bool:
    return require_int(lhs, context=context) <= require_int(rhs, context=context)


def compare_gt(lhs: Value, rhs: Value, *, context: str) -> bool:
    return require_int(lhs, context=context) > require_int(rhs, context=context)


def compare_ge(lhs: Value, rhs: Value, *, context: str) -> bool:
    return require_int(lhs, context=context) >= require_int(rhs, context=context)


def require_condition_bool(value: Value, *, context: str = "condition") -> bool:
    return require_bool(value, context=context)
