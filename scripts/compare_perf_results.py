#!/usr/bin/env python3
"""
5-Way Performance Comparison Report Generator.

Reads CSV files from a results directory (one per engine or combined)
and generates a markdown report comparing all engines.

Usage:
    python3 scripts/compare_perf_results.py [results_dir] [output]

Arguments:
    results_dir   Directory containing CSV files (default: tests/perf)
    output        Output report path (default: tests/perf/report.md)

CSV format (per file):
    timestamp,engine,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec

Legacy format (without engine column, engine inferred from filename):
    timestamp,scenario,iterations,warmup,elapsed_nanos,avg_ms,scenarios_per_sec
"""

import csv
import os
import sys
from collections import defaultdict
from pathlib import Path

SCENARIO_CATEGORIES = {
    "Size Scaling": ["layered_small", "layered_medium", "layered_large", "layered_xlarge"],
    "Algorithms": ["force_medium", "stress_medium", "mrtree_medium", "radial_medium", "rectpacking_medium"],
    "Edge Routing": ["routing_polyline", "routing_orthogonal", "routing_splines"],
    "Crossing Min": ["crossmin_layer_sweep", "crossmin_none"],
    "Hierarchy": ["hierarchy_flat", "hierarchy_nested"],
}


def categorize_scenario(name: str) -> str:
    for category, members in SCENARIO_CATEGORIES.items():
        if name in members:
            return category
    return "Other"


def parse_csv_file(filepath: str) -> list[dict]:
    """Parse a single CSV file. Handles both new (with engine) and legacy formats."""
    rows = []
    basename = Path(filepath).stem  # e.g. "rust_api", "java", "elkjs"

    with open(filepath, "r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        if reader.fieldnames is None:
            return rows

        has_engine = "engine" in reader.fieldnames
        # Handle legacy column name
        ops_col = "ops_per_sec" if "ops_per_sec" in reader.fieldnames else "scenarios_per_sec"

        for row in reader:
            try:
                entry = {
                    "engine": row.get("engine", basename) if has_engine else basename,
                    "scenario": row["scenario"],
                    "iterations": int(row.get("iterations", "1")),
                    "warmup": int(row.get("warmup", "0")),
                    "elapsed_nanos": int(row.get("elapsed_nanos", "0")),
                    "avg_ms": float(row.get("avg_ms", "0")),
                    "ops_per_sec": float(row.get(ops_col, "0")),
                }
                rows.append(entry)
            except (ValueError, KeyError):
                continue

    return rows


def load_all_results(results_dir: str) -> list[dict]:
    """Load all CSV files from the results directory."""
    all_rows = []
    csv_files = sorted(Path(results_dir).glob("*.csv"))

    if not csv_files:
        print(f"No CSV files found in {results_dir}", file=sys.stderr)
        return all_rows

    for csv_file in csv_files:
        rows = parse_csv_file(str(csv_file))
        all_rows.extend(rows)
        print(f"  Loaded {len(rows)} rows from {csv_file.name}", file=sys.stderr)

    return all_rows


def aggregate_by_engine_scenario(rows: list[dict]) -> dict:
    """
    Aggregate results: for each (engine, scenario), take the latest entry
    (highest timestamp or last in file order).
    """
    latest = {}
    for row in rows:
        key = (row["engine"], row["scenario"])
        latest[key] = row  # last wins
    return latest


def generate_report(rows: list[dict], output_path: str) -> None:
    """Generate the markdown comparison report."""
    if not rows:
        print("No data to report.", file=sys.stderr)
        return

    data = aggregate_by_engine_scenario(rows)

    # Collect unique engines and scenarios
    engines = sorted(set(r["engine"] for r in data.values()))
    scenarios = sorted(set(r["scenario"] for r in data.values()))

    # Build per-engine summary: mean avg_ms across all scenarios
    engine_summary = {}
    for engine in engines:
        engine_rows = [v for k, v in data.items() if k[0] == engine]
        if engine_rows:
            mean_ms = sum(r["avg_ms"] for r in engine_rows) / len(engine_rows)
            total_ops = sum(r["ops_per_sec"] for r in engine_rows) / len(engine_rows)
            engine_summary[engine] = {
                "count": len(engine_rows),
                "mean_ms": mean_ms,
                "mean_ops": total_ops,
            }

    # Determine baseline for "vs" column (prefer java, then rust_native)
    baseline_engine = None
    for candidate in ["java", "rust_native", "rust_api"]:
        if candidate in engine_summary:
            baseline_engine = candidate
            break
    if baseline_engine is None and engines:
        baseline_engine = engines[0]

    baseline_ms = engine_summary.get(baseline_engine, {}).get("mean_ms", 1.0) or 1.0

    # Engine display order: java first, then rust_native, rust_api, napi, wasm, elkjs
    order = ["java", "rust_native", "rust_api", "napi", "wasm", "elkjs"]
    engines_ordered = [e for e in order if e in engines]
    engines_ordered += [e for e in engines if e not in engines_ordered]

    lines = []
    lines.append("# Performance Comparison Report\n")
    lines.append("")

    # Summary table
    lines.append("## Summary\n")
    lines.append(f"| Engine | Scenarios | Avg ms (mean) | Avg ops/s | vs {baseline_engine} |")
    lines.append("|--------|-----------|---------------|-----------|{:-<{w}}|".format(
        "", w=len(baseline_engine) + 5))

    for engine in engines_ordered:
        s = engine_summary.get(engine)
        if s is None:
            continue
        ratio = baseline_ms / max(s["mean_ms"], 1e-9)
        lines.append(
            f"| {engine} | {s['count']} | {s['mean_ms']:.4f} | {s['mean_ops']:.0f} | {ratio:.2f}x |"
        )

    lines.append("")

    # Category Summary section
    lines.append("## Category Summary\n")

    cat_hdr = "| Category |"
    cat_sep = "|----------|"
    for e in engines_ordered:
        cat_hdr += f" {e}_ms |"
        cat_sep += f" {'-' * max(len(e) + 3, 8)} |"
    lines.append(cat_hdr)
    lines.append(cat_sep)

    # Build ordered list of categories (defined order, then "Other")
    category_order = list(SCENARIO_CATEGORIES.keys()) + ["Other"]
    for category in category_order:
        # Compute average avg_ms per engine for scenarios in this category
        cat_cells = f"| {category} |"
        has_any = False
        for engine in engines_ordered:
            engine_cat_rows = [
                v for k, v in data.items()
                if k[0] == engine and categorize_scenario(k[1]) == category
            ]
            if engine_cat_rows:
                avg = sum(r["avg_ms"] for r in engine_cat_rows) / len(engine_cat_rows)
                cat_cells += f" {avg:.4f} |"
                has_any = True
            else:
                cat_cells += " — |"
        if has_any:
            lines.append(cat_cells)

    lines.append("")

    # Per-scenario detail table (grouped by category)
    lines.append("## Per-Scenario Detail\n")

    # Header
    hdr = "| Scenario |"
    sep = "|----------|"
    for e in engines_ordered:
        hdr += f" {e}_ms |"
        sep += f" {'-' * max(len(e) + 3, 8)} |"
    lines.append(hdr)
    lines.append(sep)

    # Group scenarios by category, emit category header rows
    empty_cols = " |".join(" " for _ in engines_ordered)
    for category in category_order:
        cat_scenarios = [s for s in scenarios if categorize_scenario(s) == category]
        if not cat_scenarios:
            continue
        lines.append(f"| **{category}** |{empty_cols}")
        for scenario in cat_scenarios:
            row_str = f"| {scenario} |"
            for engine in engines_ordered:
                key = (engine, scenario)
                if key in data:
                    row_str += f" {data[key]['avg_ms']:.4f} |"
                else:
                    row_str += " — |"
            lines.append(row_str)

    lines.append("")

    # Per-scenario ops/s table (grouped by category)
    lines.append("## Per-Scenario Throughput (ops/s)\n")

    hdr2 = "| Scenario |"
    sep2 = "|----------|"
    for e in engines_ordered:
        hdr2 += f" {e} |"
        sep2 += f" {'-' * max(len(e), 8)} |"
    lines.append(hdr2)
    lines.append(sep2)

    for category in category_order:
        cat_scenarios = [s for s in scenarios if categorize_scenario(s) == category]
        if not cat_scenarios:
            continue
        lines.append(f"| **{category}** |{empty_cols}")
        for scenario in cat_scenarios:
            row_str = f"| {scenario} |"
            for engine in engines_ordered:
                key = (engine, scenario)
                if key in data:
                    row_str += f" {data[key]['ops_per_sec']:.0f} |"
                else:
                    row_str += " — |"
            lines.append(row_str)

    lines.append("")

    # Notes
    lines.append("## Notes\n")
    lines.append("- **elkjs** is async (Promise-based); results include Promise overhead.")
    lines.append("- **NAPI/WASM** are synchronous; WASM has one-time module init cost (absorbed in warmup).")
    lines.append("- **rust_api** measures the same code path as NAPI/WASM (JSON parse + layout + JSON serialize).")
    lines.append("- **rust_native** uses direct `ElkNode` construction (no JSON overhead).")
    lines.append(f"- Speedup ratios (vs {baseline_engine}) show how many times faster each engine is.")
    lines.append("")

    report = "\n".join(lines)

    # Write output
    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(report)

    print(f"Report written to {output_path}", file=sys.stderr)

    # Also print summary to stderr
    print("", file=sys.stderr)
    for line in lines[3:3 + len(engines_ordered) + 2]:
        print(line, file=sys.stderr)


def main():
    results_dir = sys.argv[1] if len(sys.argv) > 1 else "tests/perf"
    output_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(results_dir, "report.md")

    print(f"Loading results from {results_dir}...", file=sys.stderr)
    rows = load_all_results(results_dir)

    if not rows:
        print("No benchmark data found.", file=sys.stderr)
        sys.exit(1)

    print(f"\nTotal: {len(rows)} data points", file=sys.stderr)
    generate_report(rows, output_path)


if __name__ == "__main__":
    main()
