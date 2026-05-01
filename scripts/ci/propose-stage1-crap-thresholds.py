#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
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
