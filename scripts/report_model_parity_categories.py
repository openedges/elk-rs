#!/usr/bin/env python3
"""Generate per-category model parity reports from diff_details.tsv."""
from __future__ import annotations

import argparse
import csv
from collections import defaultdict
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Category-level model parity dashboard")
    parser.add_argument(
        "--details",
        default="perf/model_parity_full/diff_details.tsv",
        help="Path to diff_details.tsv",
    )
    parser.add_argument(
        "--out-dir",
        default="perf/model_parity_categories",
        help="Output directory for category reports",
    )
    args = parser.parse_args()

    details_path = Path(args.details)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    if not details_path.exists():
        print(f"ERROR: {details_path} not found. Run full model parity first.")
        return 1

    # Parse diff_details.tsv
    categories: dict[str, list[dict]] = defaultdict(list)
    with details_path.open("r", encoding="utf-8") as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row in reader:
            rel_path = row.get("model_rel_path", "")
            parts = rel_path.split("/")
            category = parts[0] if parts else "unknown"
            categories[category].append(row)

    # Generate per-category report
    summary_rows = []
    for cat in sorted(categories.keys()):
        models = categories[cat]
        total = len(models)
        matches = sum(1 for m in models if m.get("status") == "match")
        drifts = sum(1 for m in models if m.get("status") == "drift")
        errors = sum(1 for m in models if m.get("status") in ("error", "timeout"))
        total_diffs = sum(int(m.get("diff_count", 0)) for m in models)
        pct = f"{matches / total * 100:.1f}%" if total > 0 else "N/A"

        summary_rows.append({
            "category": cat,
            "total": total,
            "match": matches,
            "drift": drifts,
            "errors": errors,
            "total_diffs": total_diffs,
            "match_pct": pct,
        })

        # Per-category detail file
        cat_file = out_dir / f"{cat}.tsv"
        with cat_file.open("w", encoding="utf-8") as f:
            f.write("model_rel_path\tstatus\tdiff_count\ttop_category\tfirst_diff\n")
            for m in sorted(models, key=lambda x: x.get("model_rel_path", "")):
                f.write("\t".join([
                    m.get("model_rel_path", ""),
                    m.get("status", ""),
                    m.get("diff_count", "0"),
                    m.get("top_category", ""),
                    m.get("first_diff", ""),
                ]) + "\n")

    # Write summary dashboard
    dashboard = out_dir / "dashboard.md"
    with dashboard.open("w", encoding="utf-8") as f:
        f.write("# Model Parity by Category\n\n")
        f.write("| Category | Total | Match | Drift | Errors | Diffs | Match % |\n")
        f.write("|----------|-------|-------|-------|--------|-------|---------|\n")
        grand_total = grand_match = grand_drift = grand_errors = grand_diffs = 0
        for r in summary_rows:
            f.write(f"| {r['category']} | {r['total']} | {r['match']} | {r['drift']} | {r['errors']} | {r['total_diffs']} | {r['match_pct']} |\n")
            grand_total += r["total"]
            grand_match += r["match"]
            grand_drift += r["drift"]
            grand_errors += r["errors"]
            grand_diffs += r["total_diffs"]
        grand_pct = f"{grand_match / grand_total * 100:.1f}%" if grand_total > 0 else "N/A"
        f.write(f"| **Total** | **{grand_total}** | **{grand_match}** | **{grand_drift}** | **{grand_errors}** | **{grand_diffs}** | **{grand_pct}** |\n")

    # Also write summary.tsv
    with (out_dir / "summary.tsv").open("w", encoding="utf-8") as f:
        f.write("category\ttotal\tmatch\tdrift\terrors\ttotal_diffs\tmatch_pct\n")
        for r in summary_rows:
            f.write("\t".join([r["category"], str(r["total"]), str(r["match"]), str(r["drift"]), str(r["errors"]), str(r["total_diffs"]), r["match_pct"]]) + "\n")

    print(f"Category dashboard written to {dashboard}")
    for r in summary_rows:
        print(f"  {r['category']}: {r['match']}/{r['total']} ({r['match_pct']})")
    print(f"  Total: {grand_match}/{grand_total} ({grand_pct})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
