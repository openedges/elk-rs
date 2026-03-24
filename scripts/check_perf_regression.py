#!/usr/bin/env python3
"""Check benchmark output for performance regression against baseline thresholds.

Usage:
    python3 scripts/check_perf_regression.py benchmark_output.txt

Reads CSV lines from the benchmark output and checks key scenarios against
absolute thresholds. Fails (exit 1) if any scenario exceeds its threshold.

Thresholds are generous (2x baseline) to avoid false positives from CI noise.
The goal is to catch catastrophic regressions, not micro-optimizations.
"""

import sys
import csv
import io

# Absolute thresholds in ms (approximately 2x of known baseline).
# Update these when baseline changes significantly.
THRESHOLDS = {
    "layered_xlarge": 600.0,
    "layered_large": 40.0,
    "layered_medium": 8.0,
    "force_xlarge": 350.0,
    "stress_xlarge": 400.0,
    "mrtree_xlarge": 15.0,
    "radial_xlarge": 30.0,
    "crossmin_layer_sweep": 8.0,
}


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 check_perf_regression.py <benchmark_output.txt>")
        sys.exit(2)

    filepath = sys.argv[1]
    with open(filepath) as f:
        lines = f.readlines()

    # Find CSV section (starts with "timestamp,engine,scenario,...")
    csv_lines = [l for l in lines if l.startswith("1") and "," in l and "rust_native" in l]
    if not csv_lines:
        print("WARNING: No benchmark CSV data found. Skipping regression check.")
        sys.exit(0)

    header = "timestamp,engine,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec"
    reader = csv.DictReader(io.StringIO(header + "\n" + "".join(csv_lines)))

    results = {}
    for row in reader:
        scenario = row["scenario"]
        avg_ms = float(row["avg_ms"])
        results[scenario] = avg_ms

    print("=== Performance Regression Check ===")
    print(f"{'Scenario':<30} {'Measured (ms)':>15} {'Threshold (ms)':>15} {'Status':>10}")
    print("-" * 75)

    failed = False
    for scenario, threshold in sorted(THRESHOLDS.items()):
        if scenario not in results:
            print(f"{scenario:<30} {'N/A':>15} {threshold:>15.1f} {'SKIP':>10}")
            continue
        measured = results[scenario]
        status = "OK" if measured <= threshold else "FAIL"
        if status == "FAIL":
            failed = True
        print(f"{scenario:<30} {measured:>15.2f} {threshold:>15.1f} {status:>10}")

    print("-" * 75)
    if failed:
        print("REGRESSION DETECTED: One or more scenarios exceeded threshold.")
        sys.exit(1)
    else:
        print("All scenarios within threshold. No regression detected.")
        sys.exit(0)


if __name__ == "__main__":
    main()
