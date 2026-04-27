#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROUNDS = 5
BASELINE_FLOOR_MS = 50.0
COLD_BUILD_LIMIT_MULTIPLIER = 4.0
WARM_BUILD_LIMIT_MULTIPLIER = 2.0

REPO_ROOT = Path(__file__).resolve().parents[2]
HELLO_PROJECT = REPO_ROOT / "stage1/examples/hello"
AXIOMC_MANIFEST = REPO_ROOT / "stage1/Cargo.toml"
AXIOMC_BIN = REPO_ROOT / "stage1/target/debug/axiomc"
REF_DIR = REPO_ROOT / "stage1/benchmarks/reference/hello"


def run(cmd: list[str], *, cwd: Path | None = None) -> float:
    started = time.perf_counter()
    completed = subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    elapsed_ms = (time.perf_counter() - started) * 1000.0
    if completed.returncode != 0:
        if completed.stdout:
            sys.stdout.write(completed.stdout)
        if completed.stderr:
            sys.stderr.write(completed.stderr)
        raise SystemExit(completed.returncode)
    return elapsed_ms


def median_ms(samples: list[float]) -> float:
    return float(statistics.median(samples))


def collect_samples(fn, rounds: int = ROUNDS) -> tuple[list[float], float]:
    samples = [fn() for _ in range(rounds)]
    return samples, median_ms(samples)


def ensure_tools() -> None:
    required = ["cargo", "rustc", "go"]
    missing = [tool for tool in required if shutil.which(tool) is None]
    if missing:
        raise SystemExit(f"missing required benchmark tools: {', '.join(missing)}")


def build_axiomc() -> None:
    subprocess.run(
        ["cargo", "build", "--manifest-path", str(AXIOMC_MANIFEST), "-p", "axiomc"],
        cwd=REPO_ROOT,
        check=True,
    )


def axiom_cold_build() -> float:
    shutil.rmtree(HELLO_PROJECT / "dist", ignore_errors=True)
    return run([str(AXIOMC_BIN), "build", str(HELLO_PROJECT), "--json"], cwd=REPO_ROOT)


def axiom_warm_build() -> float:
    return run([str(AXIOMC_BIN), "build", str(HELLO_PROJECT), "--json"], cwd=REPO_ROOT)


def go_build(temp_dir: Path) -> float:
    output = temp_dir / "hello-go"
    output.unlink(missing_ok=True)
    return run(["go", "build", "-o", str(output), str(REF_DIR / "main.go")], cwd=temp_dir)


def rust_build(temp_dir: Path) -> float:
    output = temp_dir / "hello-rust"
    output.unlink(missing_ok=True)
    return run(["rustc", str(REF_DIR / "main.rs"), "-O", "-o", str(output)], cwd=temp_dir)


def check_limit(label: str, actual_ms: float, limit_ms: float) -> None:
    status = "PASS" if actual_ms <= limit_ms else "FAIL"
    print(f"{status} {label}: {actual_ms:.1f} ms <= {limit_ms:.1f} ms")
    if actual_ms > limit_ms:
        raise SystemExit(1)


def main() -> int:
    os.chdir(REPO_ROOT)
    ensure_tools()
    build_axiomc()

    with tempfile.TemporaryDirectory(prefix="axiom-stage1-bench-") as temp_name:
        temp_dir = Path(temp_name)

        print("warming benchmark commands...")
        axiom_cold_build()
        axiom_warm_build()
        go_build(temp_dir)
        rust_build(temp_dir)

        print("collecting benchmark medians...")
        axiom_cold_samples, axiom_cold_median = collect_samples(axiom_cold_build)
        axiom_warm_samples, axiom_warm_median = collect_samples(axiom_warm_build)
        go_samples, go_median = collect_samples(lambda: go_build(temp_dir))
        rust_samples, rust_median = collect_samples(lambda: rust_build(temp_dir))

    reference_floor = max(min(go_median, rust_median), BASELINE_FLOOR_MS)
    cold_limit = reference_floor * COLD_BUILD_LIMIT_MULTIPLIER
    warm_limit = reference_floor * WARM_BUILD_LIMIT_MULTIPLIER

    report = {
        "rounds": ROUNDS,
        "baseline_floor_ms": BASELINE_FLOOR_MS,
        "cold_build_limit_multiplier": COLD_BUILD_LIMIT_MULTIPLIER,
        "warm_build_limit_multiplier": WARM_BUILD_LIMIT_MULTIPLIER,
        "samples_ms": {
            "axiom_cold_build": axiom_cold_samples,
            "axiom_warm_build": axiom_warm_samples,
            "go_build": go_samples,
            "rust_build": rust_samples,
        },
        "medians_ms": {
            "axiom_cold_build": axiom_cold_median,
            "axiom_warm_build": axiom_warm_median,
            "go_build": go_median,
            "rust_build": rust_median,
        },
        "reference_floor_ms": reference_floor,
        "limits_ms": {
            "axiom_cold_build": cold_limit,
            "axiom_warm_build": warm_limit,
        },
    }

    print(json.dumps(report, indent=2))
    check_limit("axiom cold build", axiom_cold_median, cold_limit)
    check_limit("axiom warm build", axiom_warm_median, warm_limit)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
