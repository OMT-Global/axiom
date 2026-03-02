from __future__ import annotations


def trunc_div(a: int, b: int) -> int:
    if b == 0:
        raise ZeroDivisionError("division by zero")
    q = abs(a) // abs(b)
    if (a < 0) ^ (b < 0):
        return -q
    return q


def to_bool_int(v: int) -> int:
    return 0 if v == 0 else 1
