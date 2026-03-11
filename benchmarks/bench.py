#!/usr/bin/env python3
"""Benchmark pycg-rs against code2flow on vendored corpora."""

from __future__ import annotations

import argparse
import json
import shutil
import statistics
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

SOURCE_HINTS = {
    "black": "src/black",
    "flask": "src/flask",
    "httpx": "httpx",
    "requests": "src/requests",
    "rich": "rich",
}


def find_source_dir(corpus_root: Path, corpus_name: str) -> Path:
    candidate = corpus_root / corpus_name / SOURCE_HINTS[corpus_name]
    if not candidate.is_dir():
        raise FileNotFoundError(f"source dir not found for {corpus_name}: {candidate}")
    return candidate


def count_py_files(source_dir: Path) -> int:
    return sum(1 for _ in source_dir.rglob("*.py"))


def time_command(command: list[str], rounds: int, warmups: int) -> list[float]:
    samples: list[float] = []
    for i in range(rounds + warmups):
        start = time.perf_counter()
        completed = subprocess.run(command, capture_output=True, text=True)
        elapsed_ms = (time.perf_counter() - start) * 1000.0
        if completed.returncode != 0:
            raise RuntimeError(
                f"command failed ({completed.returncode}): {' '.join(command)}\n"
                f"{completed.stderr[:400]}"
            )
        if i >= warmups:
            samples.append(elapsed_ms)
    return samples


def summarize(samples: list[float]) -> dict[str, float]:
    return {
        "mean_ms": round(statistics.mean(samples), 2),
        "median_ms": round(statistics.median(samples), 2),
        "min_ms": round(min(samples), 2),
        "max_ms": round(max(samples), 2),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pycg", default="./target/release/pycg", help="Path to pycg binary")
    parser.add_argument(
        "--corpora", default="benchmarks/corpora", help="Path to vendored corpora"
    )
    parser.add_argument(
        "--results-dir", default="benchmarks/results", help="Directory for JSON results"
    )
    parser.add_argument("--rounds", type=int, default=5, help="Measured rounds per tool")
    parser.add_argument("--warmups", type=int, default=1, help="Warmup rounds per tool")
    args = parser.parse_args()

    corpora_root = Path(args.corpora)
    results_dir = Path(args.results_dir)
    results_dir.mkdir(parents=True, exist_ok=True)

    pycg = shutil.which(args.pycg) or args.pycg
    code2flow = shutil.which("code2flow")

    results: list[dict[str, object]] = []
    for corpus_name in SOURCE_HINTS:
        source_dir = find_source_dir(corpora_root, corpus_name)
        py_files = count_py_files(source_dir)

        pycg_cmd = [pycg, str(source_dir)]
        pycg_samples = time_command(pycg_cmd, args.rounds, args.warmups)

        corpus_result: dict[str, object] = {
            "corpus": corpus_name,
            "py_files": py_files,
            "pycg": summarize(pycg_samples),
        }

        if code2flow:
            code2flow_cmd = ["code2flow", str(source_dir)]
            code2flow_samples = time_command(code2flow_cmd, args.rounds, args.warmups)
            corpus_result["code2flow"] = summarize(code2flow_samples)

        results.append(corpus_result)

    output = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "rounds": args.rounds,
        "warmups": args.warmups,
        "results": results,
    }

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    out_path = results_dir / f"bench-{timestamp}.json"
    out_path.write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps(output, indent=2))
    print(f"\nWrote {out_path}")


if __name__ == "__main__":
    main()
