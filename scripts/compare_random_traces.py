#!/usr/bin/env python3
"""Compare Java and Rust random call traces to find the first divergence point.

Usage:
    python3 compare_random_traces.py <java_trace> <rust_trace>

Each trace file should contain lines like:
    [random #N] method() = value @ location
"""

import re
import sys
from pathlib import Path

TRACE_RE = re.compile(
    r'\[random #(\d+)\] (\w+)\(\) = (.+?) @ (.+)'
)


def parse_trace(path: Path) -> list[dict]:
    """Parse a random trace file into a list of call records."""
    records = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            m = TRACE_RE.match(line)
            if m:
                records.append({
                    'index': int(m.group(1)),
                    'method': m.group(2),
                    'value': m.group(3),
                    'location': m.group(4),
                    'raw': line,
                })
    return records


def compare_traces(java_records: list[dict], rust_records: list[dict]):
    """Compare two traces and report differences."""
    min_len = min(len(java_records), len(rust_records))

    print(f"Java trace:  {len(java_records)} calls")
    print(f"Rust trace:  {len(rust_records)} calls")
    print()

    if min_len == 0:
        print("ERROR: One or both traces are empty!")
        return

    # Find first divergence
    first_diff = None
    diff_count = 0

    for i in range(min_len):
        j = java_records[i]
        r = rust_records[i]

        method_match = j['method'] == r['method']
        value_match = j['value'] == r['value']

        if not method_match or not value_match:
            diff_count += 1
            if first_diff is None:
                first_diff = i

    if first_diff is None and len(java_records) == len(rust_records):
        print("ALL CALLS MATCH! Traces are identical.")
        print(f"  Total calls: {len(java_records)}")
        return

    if first_diff is None:
        print(f"All {min_len} common calls match, but traces differ in length.")
        print(f"  Java: {len(java_records)} calls")
        print(f"  Rust: {len(rust_records)} calls")
        return

    print(f"FIRST DIVERGENCE at call #{first_diff}")
    print(f"  Total differences: {diff_count} out of {min_len} common calls")
    print()

    # Show context around first divergence
    start = max(0, first_diff - 3)
    end = min(min_len, first_diff + 10)

    print("=== Context around first divergence ===")
    print(f"{'#':>5}  {'Match':>5}  {'Method':>12}  {'Java Value':>20}  {'Rust Value':>20}  Java Location / Rust Location")
    print("-" * 120)

    for i in range(start, end):
        j = java_records[i]
        r = rust_records[i]

        method_match = j['method'] == r['method']
        value_match = j['value'] == r['value']

        if method_match and value_match:
            marker = "  OK "
        elif not method_match:
            marker = "!METH"
        else:
            marker = "!VAL "

        highlight = " <<<" if i == first_diff else ""

        print(f"{i:>5}  {marker}  {j['method']:>12}  {j['value']:>20}  {r['value']:>20}  {j['location']} / {r['location']}{highlight}")

    print()

    # Show a few more differences after the first
    if diff_count > 1:
        print(f"=== Next differences (showing up to 10 more) ===")
        shown = 0
        for i in range(first_diff + 1, min_len):
            j = java_records[i]
            r = rust_records[i]
            if j['method'] != r['method'] or j['value'] != r['value']:
                print(f"  #{i}: Java {j['method']}()={j['value']} @ {j['location']}")
                print(f"       Rust {r['method']}()={r['value']} @ {r['location']}")
                shown += 1
                if shown >= 10:
                    remaining = diff_count - shown - 1
                    if remaining > 0:
                        print(f"  ... and {remaining} more differences")
                    break

    # Summary
    print()
    print("=== Summary ===")
    print(f"  Matching calls before divergence: {first_diff}")
    print(f"  First divergence: call #{first_diff}")
    j = java_records[first_diff]
    r = rust_records[first_diff]
    print(f"    Java: {j['method']}() = {j['value']} @ {j['location']}")
    print(f"    Rust: {r['method']}() = {r['value']} @ {r['location']}")
    print(f"  Total differences: {diff_count} / {min_len}")


def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <java_trace> <rust_trace>")
        sys.exit(1)

    java_path = Path(sys.argv[1])
    rust_path = Path(sys.argv[2])

    if not java_path.exists():
        print(f"Java trace not found: {java_path}")
        sys.exit(1)
    if not rust_path.exists():
        print(f"Rust trace not found: {rust_path}")
        sys.exit(1)

    java_records = parse_trace(java_path)
    rust_records = parse_trace(rust_path)

    compare_traces(java_records, rust_records)


if __name__ == '__main__':
    main()
