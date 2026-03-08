from __future__ import annotations

from typing import Literal

from .intops import trunc_div


Value = int | str | bool
ValueKind = Literal["int", "string", "bool", "value"]


def value_kind(value: Value) -> ValueKind:
    if type(value) is int:
        return "int"
    if type(value) is bool:
        return "bool"
    if isinstance(value, str):
        return "string"
    return "value"


def kind_matches(value: Value, expected: ValueKind) -> bool:
    return expected == "value" or value_kind(value) == expected


def render_value(value: Value) -> str:
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
