from __future__ import annotations

from difflib import get_close_matches
from pathlib import Path
from typing import Iterable, Sequence


def best_match(name: str, candidates: Iterable[str], *, cutoff: float = 0.6) -> str | None:
    unique = sorted({candidate for candidate in candidates if candidate and candidate != name})
    matches = get_close_matches(name, unique, n=1, cutoff=cutoff)
    if not matches:
        return None
    return matches[0]


def suggestion_suffix(name: str, candidates: Iterable[str], *, cutoff: float = 0.6) -> str:
    match = best_match(name, candidates, cutoff=cutoff)
    if match is None:
        return ""
    return f"; did you mean {match!r}?"


def import_path_candidates(search_paths: Sequence[Path]) -> list[str]:
    candidates: set[str] = set()
    for root in search_paths:
        if not root.exists() or not root.is_dir():
            continue
        for entry in root.iterdir():
            if not entry.is_file() or entry.suffix.lower() != ".ax":
                continue
            candidates.add(entry.name)
            candidates.add(entry.stem)
    return sorted(candidates)
