#!/usr/bin/env python3
"""Compare Java and Rust ELK layout JSON outputs from a parity manifest."""

from __future__ import annotations

import argparse
import collections
import csv
csv.field_size_limit(10 * 1024 * 1024)  # 10 MB
import json
import math
import re
from pathlib import Path
from typing import Any


def sanitize_tsv(value: str) -> str:
    return value.replace("\t", " ").replace("\n", " ").replace("\r", " ")


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def stable_array(path: str, values: list[Any]) -> list[Any]:
    if path.endswith("/sources") or path.endswith("/targets"):
        return sorted(values, key=lambda item: str(item))

    if values and all(
        isinstance(item, dict) and "id" in item and isinstance(item["id"], (str, int, float))
        for item in values
    ):
        return sorted(values, key=lambda item: str(item["id"]))

    return values


def classify_diff(diff_path: str, diff_message: str) -> str:
    """Classify a diff into a semantic category."""
    path_lower = diff_path.lower()

    if "array length mismatch" in diff_message:
        return "ordering"
    if "missing keys" in diff_message or "type mismatch" in diff_message:
        return "structure"
    if any(seg in path_lower for seg in ["/sections", "/bendpoints", "/startpoint", "/endpoint"]):
        return "section"
    if "/labels" in path_lower:
        return "label"
    if "/properties" in path_lower or "/layoutoptions" in path_lower:
        return "property"
    if any(path_lower.endswith(f"/{c}") for c in ["x", "y", "width", "height"]):
        return "coordinate"
    return "other"


def truncate_path(path: str, segments: int = 3) -> str:
    """Truncate path to first N segments, replacing array indices with [*]."""
    normalized = re.sub(r'\[\d+\]', '[*]', path)
    parts = normalized.split('/')
    return '/'.join(parts[:segments]) if len(parts) >= segments else normalized


def compare_json(
    left: Any,
    right: Any,
    path: str,
    abs_tol: float,
    max_diffs: int,
    diffs: list[str],
) -> None:
    if len(diffs) >= max_diffs:
        return

    if is_number(left) and is_number(right):
        left_num = float(left)
        right_num = float(right)
        if math.isfinite(left_num) and math.isfinite(right_num):
            if abs(left_num - right_num) <= abs_tol:
                return
        elif left_num == right_num:
            return
        diffs.append(f"{path}: number mismatch ({left_num} != {right_num})")
        return

    if type(left) is not type(right):
        diffs.append(
            f"{path}: type mismatch ({type(left).__name__} != {type(right).__name__})"
        )
        return

    if isinstance(left, dict):
        left_keys = set(left.keys())
        right_keys = set(right.keys())
        missing_left = sorted(right_keys - left_keys)
        missing_right = sorted(left_keys - right_keys)
        if missing_left:
            diffs.append(f"{path}: missing keys on left: {', '.join(missing_left)}")
            if len(diffs) >= max_diffs:
                return
        if missing_right:
            diffs.append(f"{path}: missing keys on right: {', '.join(missing_right)}")
            if len(diffs) >= max_diffs:
                return
        for key in sorted(left_keys & right_keys):
            child_path = f"{path}/{key}" if path else key
            compare_json(left[key], right[key], child_path, abs_tol, max_diffs, diffs)
            if len(diffs) >= max_diffs:
                return
        return

    if isinstance(left, list):
        left_items = stable_array(path, left)
        right_items = stable_array(path, right)
        if len(left_items) != len(right_items):
            diffs.append(
                f"{path}: array length mismatch ({len(left_items)} != {len(right_items)})"
            )
            return
        for index, (left_item, right_item) in enumerate(zip(left_items, right_items)):
            child_path = f"{path}[{index}]"
            compare_json(left_item, right_item, child_path, abs_tol, max_diffs, diffs)
            if len(diffs) >= max_diffs:
                return
        return

    if left != right:
        diffs.append(f"{path}: value mismatch ({left!r} != {right!r})")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compare Java and Rust model layout JSON manifests."
    )
    parser.add_argument("--manifest", required=True, help="Rust output manifest TSV path")
    parser.add_argument("--report", required=True, help="Markdown summary report path")
    parser.add_argument("--details", required=True, help="TSV per-model detail output path")
    parser.add_argument(
        "--abs-tol",
        type=float,
        default=1e-6,
        help="Absolute tolerance for numeric comparisons (default: 1e-6)",
    )
    parser.add_argument(
        "--max-diffs-per-model",
        type=int,
        default=20,
        help="Maximum diffs recorded per model (default: 20)",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Exit with non-zero status when drift/error exists",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    manifest_path = Path(args.manifest)
    report_path = Path(args.report)
    details_path = Path(args.details)

    if not manifest_path.exists():
        raise FileNotFoundError(f"manifest does not exist: {manifest_path}")

    details_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.parent.mkdir(parents=True, exist_ok=True)

    total = 0
    compared = 0
    matches = 0
    drift = 0
    skipped = 0
    errors = 0

    drift_rows: list[dict[str, Any]] = []
    category_counts: dict[str, int] = collections.Counter()
    path_prefix_counts: dict[str, int] = collections.Counter()
    total_diffs = 0

    with manifest_path.open("r", encoding="utf-8", newline="") as handle, details_path.open(
        "w", encoding="utf-8", newline=""
    ) as details_handle:
        reader = csv.DictReader(handle, delimiter="\t")
        required = [
            "model_rel_path",
            "java_layout_json",
            "java_status",
            "rust_layout_json",
            "rust_status",
        ]
        for column in required:
            if column not in (reader.fieldnames or []):
                raise ValueError(f"manifest missing column: {column}")

        details_writer = csv.writer(details_handle, delimiter="\t")
        details_writer.writerow(
            ["model_rel_path", "status", "diff_count", "top_category", "first_diff", "java_layout_json", "rust_layout_json"]
        )

        for row in reader:
            total += 1
            model_rel_path = row.get("model_rel_path", "")
            java_status = row.get("java_status", "")
            rust_status = row.get("rust_status", "")
            java_layout_json = row.get("java_layout_json", "")
            rust_layout_json = row.get("rust_layout_json", "")

            status = "skip"
            diff_count = 0
            first_diff = ""
            top_category = ""

            if java_status != "ok" or rust_status != "ok":
                skipped += 1
                status = "skip_non_ok"
            else:
                compared += 1
                try:
                    left = load_json(Path(java_layout_json))
                    right = load_json(Path(rust_layout_json))
                    diffs: list[str] = []
                    compare_json(left, right, "", args.abs_tol, args.max_diffs_per_model, diffs)
                    if diffs:
                        drift += 1
                        status = "drift"
                        diff_count = len(diffs)
                        first_diff = diffs[0]

                        # Classify all diffs
                        model_categories: dict[str, int] = collections.Counter()
                        for d in diffs:
                            colon_idx = d.find(": ")
                            d_path = d[:colon_idx] if colon_idx >= 0 else d
                            d_msg = d[colon_idx + 2:] if colon_idx >= 0 else d
                            cat = classify_diff(d_path, d_msg)
                            category_counts[cat] += 1
                            model_categories[cat] += 1
                            path_prefix_counts[truncate_path(d_path)] += 1
                            total_diffs += 1

                        top_category = model_categories.most_common(1)[0][0] if model_categories else ""

                        drift_rows.append(
                            {
                                "model_rel_path": model_rel_path,
                                "diff_count": str(diff_count),
                                "first_diff": first_diff,
                                "categories": dict(model_categories),
                            }
                        )
                    else:
                        matches += 1
                        status = "match"
                except Exception as exception:  # noqa: BLE001
                    errors += 1
                    status = "error"
                    first_diff = f"compare error: {exception}"

            details_writer.writerow(
                [
                    sanitize_tsv(model_rel_path),
                    status,
                    str(diff_count),
                    sanitize_tsv(top_category),
                    sanitize_tsv(first_diff),
                    sanitize_tsv(java_layout_json),
                    sanitize_tsv(rust_layout_json),
                ]
            )

    with report_path.open("w", encoding="utf-8") as report:
        report.write("# ELK Model Parity Report\n\n")
        report.write(f"- manifest: `{manifest_path}`\n")
        report.write(f"- total rows: {total}\n")
        report.write(f"- compared rows: {compared}\n")
        report.write(f"- matched rows: {matches}\n")
        report.write(f"- drift rows: {drift}\n")
        report.write(f"- skipped rows (java/rust non-ok): {skipped}\n")
        report.write(f"- compare errors: {errors}\n")
        report.write(f"- abs tolerance: {args.abs_tol}\n")
        report.write(f"- max diffs per model: {args.max_diffs_per_model}\n")
        report.write(f"- total diffs across all models: {total_diffs}\n")
        report.write("\n")

        report.write("## Drift Classification Summary\n\n")
        if total_diffs > 0:
            report.write("| Category | Count | Percentage |\n")
            report.write("|----------|------:|-----------:|\n")
            for cat, count in sorted(category_counts.items(), key=lambda x: -x[1]):
                pct = 100.0 * count / total_diffs
                report.write(f"| {cat} | {count} | {pct:.1f}% |\n")
            report.write("\n")

            report.write("### Top Diff Path Prefixes\n\n")
            for prefix, count in path_prefix_counts.most_common(10):
                pct = 100.0 * count / total_diffs
                report.write(f"- `{prefix}`: {count} ({pct:.1f}%)\n")
        else:
            report.write("- no diffs\n")
        report.write("\n")

        report.write("## Drift Samples\n\n")
        if drift_rows:
            for row in drift_rows[:20]:
                cats = row.get("categories", {})
                cat_str = ", ".join(f"{k}={v}" for k, v in sorted(cats.items(), key=lambda x: -x[1]))
                report.write(
                    f"- `{row['model_rel_path']}`: diffs={row['diff_count']} [{cat_str}], first: {row['first_diff']}\n"
                )
        else:
            report.write("- none\n")

    print(
        "model parity compare summary: "
        f"total={total}, compared={compared}, matches={matches}, drift={drift}, "
        f"skipped={skipped}, errors={errors}, total_diffs={total_diffs}"
    )

    if args.strict and (drift > 0 or errors > 0):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
