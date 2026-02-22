#!/usr/bin/env python3
"""Summarize layered phase-gate results from manifests, traces, and batch compare JSON.

Rules implemented:
1. Base model set = java_manifest rows with java_status == "ok".
2. Missing Java/Rust trace for a base model => precheck error (comparison unavailable).
3. For comparable models, only the first non-match step is counted as error.
4. Steps after first error are not judged.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class StepGateStat:
    reached: int = 0
    match: int = 0
    error: int = 0
    processor: str = ""


def load_tsv(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def find_trace_models(base: Path) -> set[str]:
    models: set[str] = set()
    for dirpath, _dirnames, filenames in os.walk(base):
        if any(name.startswith("step_") and name.endswith(".json") for name in filenames):
            rel = os.path.relpath(dirpath, base).replace("\\", "/")
            models.add(rel.replace(".json", ""))
    return models


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize phase-gate status with precheck and first-failure step stats."
    )
    parser.add_argument("--java-manifest", type=Path, required=True)
    parser.add_argument("--rust-manifest", type=Path, required=True)
    parser.add_argument("--java-trace-dir", type=Path, required=True)
    parser.add_argument("--rust-trace-dir", type=Path, required=True)
    parser.add_argument("--compare-json", type=Path, required=True)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-md", type=Path, required=True)
    parser.add_argument("--max-listed-models", type=int, default=50)
    return parser.parse_args()


def ensure_parent(path: Path) -> None:
    if path.parent and not path.parent.exists():
        path.parent.mkdir(parents=True, exist_ok=True)


def summarize(args: argparse.Namespace) -> dict[str, Any]:
    java_rows = load_tsv(args.java_manifest)
    _rust_rows = load_tsv(args.rust_manifest)

    base_models = [row["model_rel_path"] for row in java_rows if row.get("java_status") == "ok"]
    base_model_set = set(base_models)

    java_trace_models = find_trace_models(args.java_trace_dir)
    rust_trace_models = find_trace_models(args.rust_trace_dir)

    missing_java_trace = sorted(
        model
        for model in base_models
        if model not in java_trace_models and model in rust_trace_models
    )
    missing_rust_trace = sorted(
        model
        for model in base_models
        if model in java_trace_models and model not in rust_trace_models
    )
    missing_both_trace = sorted(
        model
        for model in base_models
        if model not in java_trace_models and model not in rust_trace_models
    )

    precheck_error_models = (
        missing_java_trace + missing_rust_trace + missing_both_trace
    )
    comparable_models = sorted(
        model
        for model in base_model_set
        if model in java_trace_models and model in rust_trace_models
    )

    with args.compare_json.open("r", encoding="utf-8") as handle:
        compare_payload = json.load(handle)

    compare_models = {
        entry["model"]: entry for entry in compare_payload.get("models", [])
    }
    missing_compare_entry = sorted(
        model for model in comparable_models if model not in compare_models
    )
    # Compare-missing is also a precheck-level error (comparison unavailable).
    precheck_error_models.extend(missing_compare_entry)

    first_failure_counter: Counter[int] = Counter()
    step_gate_stats: dict[int, StepGateStat] = {}
    all_match_models = 0

    for model in comparable_models:
        entry = compare_models.get(model)
        if entry is None:
            continue

        first_failed = False
        for step_entry in entry.get("steps", []):
            step = step_entry.get("step")
            if step is None:
                continue
            step = int(step)
            status = step_entry.get("status", "missing")
            processor = step_entry.get("processor", "")
            stat = step_gate_stats.setdefault(step, StepGateStat())
            stat.reached += 1
            if not stat.processor and processor:
                stat.processor = processor

            if status == "match":
                stat.match += 1
            else:
                stat.error += 1
                first_failure_counter[step] += 1
                first_failed = True
                break

        if not first_failed:
            all_match_models += 1

    sorted_steps = sorted(step_gate_stats.keys())
    step_rows = []
    for step in sorted_steps:
        stat = step_gate_stats[step]
        step_rows.append(
            {
                "step": step,
                "processor": stat.processor,
                "reached": stat.reached,
                "match": stat.match,
                "error": stat.error,
            }
        )

    precheck_errors = len(precheck_error_models)
    gate_pass = precheck_errors == 0 and all(row["error"] == 0 for row in step_rows)

    summary = {
        "base_models": len(base_models),
        "java_trace_models": len(java_trace_models),
        "rust_trace_models": len(rust_trace_models),
        "comparable_models": len(comparable_models),
        "all_match_models": all_match_models,
        "diverged_models": len(comparable_models) - all_match_models,
        "precheck_errors": precheck_errors,
        "precheck": {
            "missing_java_trace": len(missing_java_trace),
            "missing_rust_trace": len(missing_rust_trace),
            "missing_both_trace": len(missing_both_trace),
            "missing_compare_entry": len(missing_compare_entry),
            "missing_java_trace_models": missing_java_trace,
            "missing_rust_trace_models": missing_rust_trace,
            "missing_both_trace_models": missing_both_trace,
            "missing_compare_entry_models": missing_compare_entry,
        },
        "first_failure_by_step": {
            str(step): count for step, count in sorted(first_failure_counter.items())
        },
        "step_gate": step_rows,
        "gate_pass": gate_pass,
        "inputs": {
            "java_manifest": str(args.java_manifest),
            "rust_manifest": str(args.rust_manifest),
            "java_trace_dir": str(args.java_trace_dir),
            "rust_trace_dir": str(args.rust_trace_dir),
            "compare_json": str(args.compare_json),
        },
    }
    return summary


def write_markdown(summary: dict[str, Any], output: Path, max_listed_models: int) -> None:
    precheck = summary["precheck"]
    lines: list[str] = []
    lines.append("# Layered Phase-Gate Summary")
    lines.append("")
    lines.append(f"- gate_pass: **{str(summary['gate_pass']).lower()}**")
    lines.append(f"- base_models(java_status=ok): **{summary['base_models']}**")
    lines.append(f"- comparable_models: **{summary['comparable_models']}**")
    lines.append(f"- precheck_errors(비교불가): **{summary['precheck_errors']}**")
    lines.append(
        f"- all_match_models: **{summary['all_match_models']}**, diverged_models: **{summary['diverged_models']}**"
    )
    lines.append("")
    lines.append("## Precheck")
    lines.append("")
    lines.append(
        f"- missing_java_trace: {precheck['missing_java_trace']}"
    )
    lines.append(
        f"- missing_rust_trace: {precheck['missing_rust_trace']}"
    )
    lines.append(
        f"- missing_both_trace: {precheck['missing_both_trace']}"
    )
    lines.append(
        f"- missing_compare_entry: {precheck['missing_compare_entry']}"
    )

    def append_model_list(title: str, models: list[str]) -> None:
        if not models:
            return
        lines.append("")
        lines.append(f"### {title}")
        lines.append("")
        for model in models[:max_listed_models]:
            lines.append(f"- {model}")
        remaining = len(models) - max_listed_models
        if remaining > 0:
            lines.append(f"- ... and {remaining} more")

    append_model_list("missing_java_trace_models", precheck["missing_java_trace_models"])
    append_model_list("missing_rust_trace_models", precheck["missing_rust_trace_models"])
    append_model_list("missing_both_trace_models", precheck["missing_both_trace_models"])
    append_model_list("missing_compare_entry_models", precheck["missing_compare_entry_models"])

    lines.append("")
    lines.append("## Phase Gate")
    lines.append("")
    lines.append("| step | processor | reached | match | error |")
    lines.append("| ---: | --- | ---: | ---: | ---: |")
    for row in summary["step_gate"]:
        lines.append(
            f"| {row['step']} | {row['processor']} | {row['reached']} | {row['match']} | {row['error']} |"
        )

    lines.append("")
    lines.append("## First Failure By Step")
    lines.append("")
    if not summary["first_failure_by_step"]:
        lines.append("- (none)")
    else:
        for step, count in summary["first_failure_by_step"].items():
            lines.append(f"- step {step}: {count}")

    ensure_parent(output)
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    summary = summarize(args)

    ensure_parent(args.output_json)
    args.output_json.write_text(
        json.dumps(summary, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    write_markdown(summary, args.output_md, args.max_listed_models)

    print(
        "phase gate summary written: "
        f"json={args.output_json} md={args.output_md} "
        f"precheck_errors={summary['precheck_errors']} comparable={summary['comparable_models']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
