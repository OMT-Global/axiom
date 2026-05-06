#!/usr/bin/env python3
"""Propose non-blocking CRAP thresholds for stage1 Rust hotspots.

The script intentionally reports only. It can rank hotspots from source alone and,
when an LCOV report is supplied, folds line coverage into the CRAP formula:

    CRAP = complexity^2 * (1 - coverage)^3 + complexity

Use --enforce only after the proposal is accepted; the default exit code is zero.
"""

from __future__ import annotations

import argparse
import json

import math
import re
import statistics
import sys
from dataclasses import dataclass, asdict
from pathlib import Path

DEFAULT_WATCH = 30.0
DEFAULT_WARN = 60.0
DEFAULT_CRITICAL = 100.0
DECISION_RE = re.compile(
    r"\b(if|else\s+if|match|while|for|loop)\b|&&|\|\||\?"
)
FN_RE = re.compile(r"^(?P<indent>\s*)(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b")

import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_ROOT = REPO_ROOT / "stage1/crates/axiomc/src"
DEFAULT_THRESHOLD = 30.0
BRANCH_TOKENS = ("if ", "if(", "match ", "for ", "while ", "&&", "||", "?")

@dataclass(frozen=True)
class FunctionMetric:
    path: str
    name: str
    start_line: int
    end_line: int
    lines: int
    complexity: int
    coverage_percent: float | None
    crap: float

def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]

def code_chars(line: str) -> str:
    """Return a line with string/char literals and line comments neutralized."""
    out: list[str] = []
    index = 0
    in_string = False
    in_char = False
    escape = False
    while index < len(line):
        ch = line[index]
        nxt = line[index + 1] if index + 1 < len(line) else ""
        if not in_string and not in_char and ch == "/" and nxt == "/":
            break
        if escape:
            escape = False
            out.append(" ")
        elif ch == "\\" and (in_string or in_char):
            escape = True
            out.append(" ")
        elif ch == '"' and not in_char:
            in_string = not in_string
            out.append(" ")
        elif ch == "'" and not in_string:
            # Keep lifetime markers such as 'a intact enough not to matter for
            # brace counting, but neutralize ordinary char literals.
            if nxt and not (nxt.isalpha() or nxt == "_"):
                in_char = not in_char
            out.append(" ")
        elif in_string or in_char:
            out.append(" ")
        else:
            out.append(ch)
        index += 1
    return "".join(out)

def count_delta(line: str) -> int:
    code = code_chars(line)
    return code.count("{") - code.count("}")

def complexity_for(lines: list[str]) -> int:
    total = 1
    for line in lines:
        total += len(DECISION_RE.findall(code_chars(line)))
    return total

def parse_functions(path: Path, root: Path) -> list[tuple[str, str, int, int, list[str]]]:
    text = path.read_text(encoding="utf-8").splitlines()
    functions: list[tuple[str, str, int, int, list[str]]] = []
    index = 0
    while index < len(text):
        match = FN_RE.match(text[index])
        if not match:
            index += 1
            continue
        name = match.group("name")
        start = index + 1
        brace_depth = 0
        body: list[str] = []
        seen_open = False
        cursor = index
        while cursor < len(text):
            line = text[cursor]
            body.append(line)
            delta = count_delta(line)
            if "{" in line:
                seen_open = True
            brace_depth += delta
            if seen_open and brace_depth <= 0:
                break
            cursor += 1
        end = min(cursor + 1, len(text))
        rel = path.relative_to(root).as_posix()
        functions.append((rel, name, start, end, body))
        index = cursor + 1
    return functions

def parse_lcov(path: Path | None, root: Path) -> dict[str, dict[int, int]]:
    if path is None:
        return {}
    coverage: dict[str, dict[int, int]] = {}
    current: str | None = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        if raw.startswith("SF:"):
            source = Path(raw[3:])
            try:
                current = source.resolve().relative_to(root.resolve()).as_posix()
            except ValueError:
                current = source.as_posix()
            coverage.setdefault(current, {})
        elif raw.startswith("DA:") and current is not None:
            number, hits, *_ = raw[3:].split(",")
            coverage[current][int(number)] = int(hits)
        elif raw == "end_of_record":
            current = None
    return coverage

def function_coverage(path: str, start: int, end: int, lcov: dict[str, dict[int, int]]) -> float | None:
    lines = lcov.get(path)
    if not lines:
        return None
    executable = [line for line in range(start, end + 1) if line in lines]
    if not executable:
        return None
    covered = sum(1 for line in executable if lines[line] > 0)
    return covered / len(executable) * 100.0

def crap_score(complexity: int, coverage_percent: float | None) -> float:
    coverage = 0.0 if coverage_percent is None else coverage_percent / 100.0
    return complexity**2 * (1.0 - coverage) ** 3 + complexity

def percentile(values: list[float], pct: float) -> float:
    if not values:
        return 0.0
    if len(values) == 1:
        return values[0]
    return float(statistics.quantiles(values, n=100, method="inclusive")[int(pct) - 1])

def propose(metrics: list[FunctionMetric], *, watch_floor: float) -> dict:
    scores = [metric.crap for metric in metrics]
    p90 = percentile(scores, 90)
    p95 = percentile(scores, 95)
    p99 = percentile(scores, 99)
    observed_max = max(scores, default=0.0)
    baseline_watch = max(watch_floor, math.ceil(p95 / 5.0) * 5.0)
    baseline_warn = max(DEFAULT_WARN, math.ceil(max(p99, baseline_watch * 1.5) / 5.0) * 5.0)
    baseline_critical = max(DEFAULT_CRITICAL, math.ceil(max(observed_max, baseline_warn * 1.5) / 5.0) * 5.0)
    return {
        "schemaVersion": 1,
        "status": "proposal-only",
        "ciBlocking": False,
        "formula": "complexity^2 * (1 - coverage)^3 + complexity",
        "coverage": {
            "input": "lcov" if any(m.coverage_percent is not None for m in metrics) else None,
            "missingCoveragePolicy": "rank as 0% covered, do not enforce",
        },
        "summary": {
            "functionsAnalyzed": len(metrics),
            "p90": round(p90, 2),
            "p95": round(p95, 2),
            "p99": round(p99, 2),
            "max": round(observed_max, 2),
        },
        "proposedThresholds": {
            "watch": DEFAULT_WATCH,
            "warn": DEFAULT_WARN,
            "critical": DEFAULT_CRITICAL,
        },
        "observedBootstrapThresholds": {
            "watch": baseline_watch,
            "warn": baseline_warn,
            "critical": baseline_critical,
        },
        "ratchetPolicy": "initial CI opt-in should report all hotspots and fail only new or changed functions above proposedThresholds unless maintainers explicitly choose a full-baseline cleanup gate",
        "enablement": {
            "defaultMode": "report-only",
            "blockingMode": "requires explicit --enforce or CI opt-in after acceptance",
        },
        "hotspots": [asdict(metric) for metric in sorted(metrics, key=lambda item: item.crap, reverse=True)[:20]],
    }

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=repo_root() / "stage1/crates/axiomc/src")
    parser.add_argument("--lcov", type=Path, default=None, help="Optional LCOV report to incorporate line coverage")
    parser.add_argument("--output", type=Path, default=None, help="Write proposal JSON to this path")
    parser.add_argument("--watch-threshold", type=float, default=DEFAULT_WATCH)
    parser.add_argument("--enforce", action="store_true", help="Fail if any function exceeds the watch threshold")
    args = parser.parse_args(argv)

    root = repo_root()
    source_root = args.source_root.resolve()
    lcov = parse_lcov(args.lcov, root)
    metrics: list[FunctionMetric] = []
    for path in sorted(source_root.rglob("*.rs")):
        for rel, name, start, end, body in parse_functions(path, root):
            coverage = function_coverage(rel, start, end, lcov)
            complexity = complexity_for(body)
            metrics.append(
                FunctionMetric(
                    path=rel,
                    name=name,
                    start_line=start,
                    end_line=end,
                    lines=end - start + 1,
                    complexity=complexity,
                    coverage_percent=None if coverage is None else round(coverage, 2),
                    crap=round(crap_score(complexity, coverage), 2),
                )
            )

    proposal = propose(metrics, watch_floor=args.watch_threshold)
    payload = json.dumps(proposal, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(payload, encoding="utf-8")
    else:
        sys.stdout.write(payload)

    if args.enforce:
        offenders = [metric for metric in metrics if metric.crap > args.watch_threshold]
        if offenders:
            print(
                f"CRAP watch threshold exceeded by {len(offenders)} function(s); "
                "this mode is opt-in and should not be wired to CI until accepted.",
                file=sys.stderr,
            )
            return 1
    name: str
    path: Path
    line: int
    complexity: int
    coverage: float

    @property
    def crap(self) -> float:
        uncovered = 1.0 - self.coverage
        return (self.complexity**2 * uncovered**3) + self.complexity

def function_ranges(source: str) -> list[tuple[str, int, str]]:
    matches = list(
        re.finditer(
            r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(?:async|const|unsafe)\s+)*fn\s+([A-Za-z_][A-Za-z0-9_]*)[^{;]*\{",
            source,
        )
    )
    ranges: list[tuple[str, int, str]] = []
    for index, match in enumerate(matches):
        start = match.start()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(source)
        line = source.count("\n", 0, start) + 1
        ranges.append((match.group(1), line, source[start:end]))
    return ranges

def cyclomatic_complexity(body: str) -> int:
    complexity = 1
    for token in BRANCH_TOKENS:
        complexity += body.count(token)
    complexity += body.count("=>")
    return complexity

def collect_metrics(source_root: Path, default_coverage: float) -> list[FunctionMetric]:
    metrics: list[FunctionMetric] = []
    for path in sorted(source_root.rglob("*.rs")):
        source = path.read_text(encoding="utf-8")
        for name, line, body in function_ranges(source):
            metrics.append(
                FunctionMetric(
                    name=name,
                    path=path,
                    line=line,
                    complexity=cyclomatic_complexity(body),
                    coverage=default_coverage,
                )
            )
    return metrics

def proposal(metrics: list[FunctionMetric], threshold: float, max_hotspots: int, source_root: Path) -> dict:
    hotspots = sorted(metrics, key=lambda metric: metric.crap, reverse=True)[:max_hotspots]
    return {
        "schema_version": "axiom.stage1.crap-threshold-proposal.v1",
        "blocking": False,
        "source_root": str(source_root),
        "threshold": threshold,
        "inputs": {
            "coverage": "defaulted until coverage artifacts are wired into extended validation",
            "complexity": "heuristic branch-token scan over stage1 Rust sources",
        },
        "summary": {
            "functions_scanned": len(metrics),
            "hotspots_over_threshold": sum(1 for metric in metrics if metric.crap > threshold),
            "max_crap": max((metric.crap for metric in metrics), default=0.0),
        },
        "hotspots": [
            {
                "function": metric.name,
                "path": str(metric.path),
                "line": metric.line,
                "complexity": metric.complexity,
                "coverage": metric.coverage,
                "crap": round(metric.crap, 2),
                "over_threshold": metric.crap > threshold,
            }
            for metric in hotspots
        ],
        "proposed_policy": {
            "warn_threshold": threshold,
            "blocking_threshold": None,
            "enable_blocking_by": "rerun with --enforce after coverage artifacts and baselines are stable",
        },
    }

def main() -> int:
    parser = argparse.ArgumentParser(description="Propose non-blocking CRAP thresholds for stage1 hotspots.")
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD)
    parser.add_argument("--default-coverage", type=float, default=0.0)
    parser.add_argument("--max-hotspots", type=int, default=20)
    parser.add_argument("--enforce", action="store_true")
    args = parser.parse_args()

    metrics = collect_metrics(args.source_root, args.default_coverage)
    report = proposal(metrics, args.threshold, args.max_hotspots, args.source_root)
    print(json.dumps(report, indent=2))

    if args.enforce and report["summary"]["hotspots_over_threshold"] > 0:
        return 1
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
