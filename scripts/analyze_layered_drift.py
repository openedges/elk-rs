#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Dict, Iterable, List, Tuple


def load_json(path: Path) -> dict:
    with path.open() as handle:
        return json.load(handle)


def is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def compare_json(
    left: object,
    right: object,
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
        if math_is_finite(left_num) and math_is_finite(right_num):
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
            return
        if missing_right:
            diffs.append(f"{path}: missing keys on right: {', '.join(missing_right)}")
            return

        for key in sorted(left_keys & right_keys):
            child_path = f"{path}/{key}" if path else key
            compare_json(left[key], right[key], child_path, abs_tol, max_diffs, diffs)
            if len(diffs) >= max_diffs:
                return
        return

    if isinstance(left, list):
        if len(left) != len(right):
            diffs.append(f"{path}: array length mismatch ({len(left)} != {len(right)})")
            return

        for index, (left_item, right_item) in enumerate(zip(left, right)):
            child_path = f"{path}[{index}]"
            compare_json(left_item, right_item, child_path, abs_tol, max_diffs, diffs)
            if len(diffs) >= max_diffs:
                return
        return

    if left != right:
        diffs.append(f"{path}: value mismatch ({left!r} != {right!r})")


def math_is_finite(value: float) -> bool:
    return value == value and abs(value) != float("inf")


def get_direction(input_json: dict) -> str:
    return input_json.get("layoutOptions", {}).get("org.eclipse.elk.direction", "RIGHT")


def layer_axis(direction: str) -> Tuple[str, str]:
    direction = direction.upper()
    if direction in ("RIGHT", "LEFT"):
        return "x", "y"
    return "y", "x"


def layer_index_by_coord(nodes: Iterable[dict], primary_key: str, tol: float = 1e-6):
    coords = sorted({n[primary_key] for n in nodes if primary_key in n})
    uniq: List[float] = []
    for coord in coords:
        if not uniq or abs(coord - uniq[-1]) > tol:
            uniq.append(coord)

    def idx(coord: float) -> int:
        best = 0
        best_distance = None
        for i, u in enumerate(uniq):
            dist = abs(coord - u)
            if best_distance is None or dist < best_distance:
                best_distance = dist
                best = i
        return best

    return idx


def classify_model(input_path: Path, java_path: Path, rust_path: Path) -> str:
    inp = load_json(input_path)
    java = load_json(java_path)
    rust = load_json(rust_path)

    primary, secondary = layer_axis(get_direction(inp))

    java_nodes = [n for n in java.get("children", []) if isinstance(n, dict) and "id" in n]
    rust_nodes = [n for n in rust.get("children", []) if isinstance(n, dict) and "id" in n]
    java_map = {n["id"]: n for n in java_nodes}
    rust_map = {n["id"]: n for n in rust_nodes}
    common_ids = [node_id for node_id in java_map if node_id in rust_map]
    if not common_ids:
        return "other"

    java_idx = layer_index_by_coord((java_map[n] for n in common_ids), primary)
    rust_idx = layer_index_by_coord((rust_map[n] for n in common_ids), primary)

    for node_id in common_ids:
        if primary not in java_map[node_id] or primary not in rust_map[node_id]:
            continue
        if java_idx(java_map[node_id][primary]) != rust_idx(rust_map[node_id][primary]):
            return "layering_diff"

    def build_order(node_map: Dict[str, dict], idx_fn) -> Dict[int, List[str]]:
        layers: Dict[int, List[Tuple[float, str]]] = defaultdict(list)
        for node_id in common_ids:
            node = node_map[node_id]
            if primary not in node or secondary not in node:
                continue
            layers[idx_fn(node[primary])].append((node[secondary], node_id))
        return {layer: [node_id for _, node_id in sorted(items)] for layer, items in layers.items()}

    java_layers = build_order(java_map, java_idx)
    rust_layers = build_order(rust_map, rust_idx)
    for layer_idx, java_order in java_layers.items():
        rust_order = rust_layers.get(layer_idx)
        if rust_order is None:
            continue
        if java_order != rust_order:
            return "ordering_diff"

    return "other"


def normalize_diff_path(path: str) -> str:
    if not path:
        return path
    import re

    return re.sub(r"\[\d+\]", "[*]", path)


def classify_path_phase(path: str) -> str:
    path_lower = path.lower()
    if "/sections" in path_lower or "/bendpoints" in path_lower:
        return "edge_routing_sections"
    if "/edges" in path_lower:
        return "edge_routing_edges"
    if path_lower.endswith("/labels") or "/labels" in path_lower:
        return "label"
    if "/ports" in path_lower:
        return "ports"
    if path_lower.endswith("/x") or path_lower.endswith("/y") or path_lower.endswith("/width") or path_lower.endswith("/height"):
        return "node_coordinates"
    if "/properties" in path_lower or "/layoutoptions" in path_lower or "/options" in path_lower:
        return "metadata"
    if "structure" in path_lower or "missing keys" in path_lower:
        return "structure"
    return "other"


PHASE_ROOT_ORDER = [
    "p2_layering",
    "p3_crossing_order",
    "p4_node_placement",
    "p4_label_management",
    "p5_edge_routing",
    "import_or_structure",
    "unknown",
]

SYMPTOM_TO_PHASE_ROOT = {
    "node_coordinates": "p4_node_placement",
    "ports": "p4_node_placement",
    "label": "p4_label_management",
    "edge_routing_sections": "p5_edge_routing",
    "edge_routing_edges": "p5_edge_routing",
    "metadata": "import_or_structure",
    "structure": "import_or_structure",
    "other": "unknown",
}

DIFF_BUCKET_RANK = {"low_1_5": 0, "medium_6_19": 1, "high_20_cap": 2}


def diff_bucket(diff_count: int) -> str:
    if diff_count <= 5:
        return "low_1_5"
    if diff_count <= 19:
        return "medium_6_19"
    return "high_20_cap"


def counter_to_compact(counter: Counter) -> str:
    return ";".join(f"{key}:{value}" for key, value in counter.most_common())


def infer_phase_root(
    model_class: str,
    top_category: str,
    symptoms: Counter,
) -> Tuple[str, str]:
    if model_class == "layering_diff":
        return "p2_layering", "layer index differs between Java and Rust"
    if model_class == "ordering_diff":
        return "p3_crossing_order", "in-layer order differs between Java and Rust"

    category = (top_category or "").lower()
    if category == "section":
        return "p5_edge_routing", "top category=section"
    if category == "label":
        return "p4_label_management", "top category=label"
    if category == "ordering":
        return "p3_crossing_order", "top category=ordering"

    votes = Counter()
    for symptom, count in symptoms.items():
        votes[SYMPTOM_TO_PHASE_ROOT.get(symptom, "unknown")] += count

    if category == "coordinate":
        votes["p4_node_placement"] += 2
    if category == "structure":
        votes["import_or_structure"] += 2

    if not votes:
        return "unknown", "no sampled diffs"

    phase_root, _ = votes.most_common(1)[0]
    return phase_root, f"symptom votes={dict(votes)}"


def load_manifest(path: Path) -> Dict[str, dict]:
    with path.open() as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        return {row["model_rel_path"]: row for row in reader}


def load_drift_rows(path: Path) -> List[dict]:
    with path.open() as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        rows = []
        for row in reader:
            if row["status"] == "drift":
                row["diff_count"] = int(row["diff_count"])
                rows.append(row)
        return rows


def collect_diff_paths(
    java_path: Path,
    rust_path: Path,
    max_diffs: int,
    abs_tol: float,
) -> list[str]:
    left = load_json(java_path)
    right = load_json(rust_path)
    diffs: list[str] = []
    compare_json(left, right, "", abs_tol, max_diffs, diffs)
    return diffs


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Heuristic drift classifier for layered parity outputs (top-level only)."
    )
    parser.add_argument(
        "--diff-details",
        default="perf/model_parity_full/diff_details.tsv",
        help="Path to diff_details.tsv",
    )
    parser.add_argument(
        "--manifest",
        default="perf/model_parity_full/rust_manifest.tsv",
        help="Path to rust_manifest.tsv",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=15,
        help="Number of sample rows to show per class",
    )
    parser.add_argument(
        "--max-diffs",
        type=int,
        default=40,
        help="Number of diffs inspected per model for phase inference",
    )
    parser.add_argument(
        "--abs-tol",
        type=float,
        default=1e-6,
        help="Absolute tolerance for numeric comparisons",
    )
    parser.add_argument(
        "--prefix",
        action="append",
        default=[],
        help="Optional model path prefix filter (can be specified multiple times)",
    )
    parser.add_argument(
        "--out-summary-md",
        default="perf/model_parity_full/phase_root_summary.md",
        help="Markdown output path for phase-root summary",
    )
    parser.add_argument(
        "--out-models-tsv",
        default="perf/model_parity_full/phase_root_models.tsv",
        help="TSV output path for per-model phase-root classification",
    )
    parser.add_argument(
        "--focus-top-phases",
        type=int,
        default=2,
        help="How many top phase roots to extract for focused fixing",
    )
    parser.add_argument(
        "--out-focus-tsv",
        default="perf/model_parity_full/phase_focus_top.tsv",
        help="TSV output path for models in top phase roots",
    )
    parser.add_argument(
        "--out-focus-md",
        default="perf/model_parity_full/phase_focus_top.md",
        help="Markdown output path for top phase focus summary",
    )
    args = parser.parse_args()

    manifest = load_manifest(Path(args.manifest))
    drift_rows = load_drift_rows(Path(args.diff_details))

    if args.prefix:
        drift_rows = [
            r
            for r in drift_rows
            if any(r["model_rel_path"].startswith(prefix) for prefix in args.prefix)
        ]

    classified: Dict[str, List[dict]] = defaultdict(list)
    phase_counter: Dict[str, Counter] = defaultdict(Counter)
    prefix_counter: Dict[str, Counter] = defaultdict(Counter)
    phase_root_counter: Counter = Counter()
    phase_root_diff_counter: Counter = Counter()
    phase_root_bucket_counter: Dict[str, Counter] = defaultdict(Counter)
    phase_root_category_counter: Dict[str, Counter] = defaultdict(Counter)
    phase_root_prefix_counter: Dict[str, Counter] = defaultdict(Counter)
    phase_root_rows: Dict[str, List[dict]] = defaultdict(list)

    for row in drift_rows:
        manifest_row = manifest.get(row["model_rel_path"])
        if not manifest_row:
            continue

        cls = classify_model(
            Path(manifest_row["input_json"]),
            Path(manifest_row["java_layout_json"]),
            Path(manifest_row["rust_layout_json"]),
        )

        diffs = collect_diff_paths(
            Path(manifest_row["java_layout_json"]),
            Path(manifest_row["rust_layout_json"]),
            max_diffs=args.max_diffs,
            abs_tol=args.abs_tol,
        )
        phases = Counter()
        for diff in diffs:
            path = diff.split(": ", 1)[0]
            normalized = normalize_diff_path(path)
            phases[classify_path_phase(path)] += 1
            phase_counter[cls][classify_path_phase(path)] += 1
            prefix_counter[cls][normalized] += 1

        phase_root, phase_reason = infer_phase_root(
            cls,
            row.get("top_category", ""),
            phases,
        )
        bucket = diff_bucket(row["diff_count"])

        row = dict(row)
        row["class"] = cls
        row["phases"] = phases
        row["phase_signature"] = ";".join(
            phase for phase, _ in phases.most_common()
        )
        row["phase_root"] = phase_root
        row["phase_reason"] = phase_reason
        row["diff_bucket"] = bucket
        classified[cls].append(row)
        phase_root_rows[phase_root].append(row)
        phase_root_counter[phase_root] += 1
        phase_root_diff_counter[phase_root] += row["diff_count"]
        phase_root_bucket_counter[phase_root][bucket] += 1
        phase_root_category_counter[phase_root][row.get("top_category", "unknown")] += 1
        model_prefix = row["model_rel_path"].split("/")[0]
        phase_root_prefix_counter[phase_root][model_prefix] += 1

    total = sum(len(rows) for rows in classified.values())
    print(f"Drift models classified: {total}")
    counts = Counter({cls: len(rows) for cls, rows in classified.items()})
    for cls, count in counts.most_common():
        print(f"  {cls}: {count}")

    # prefix by top-level
    for cls, rows in classified.items():
        print(f"\n[ {cls} ]")
        if not rows:
            print("  none")
            continue
        print("  phase counts")
        for phase, count in phase_counter[cls].most_common():
            print(f"    - {phase}: {count}")

    # Breakdown by first diff path
    for cls, counter in phase_counter.items():
        if not counter:
            continue
        print(f"\nTop prefix/path hints for {cls}")
        for prefix, count in prefix_counter[cls].most_common(12):
            print(f"  - {prefix}: {count}")

    # Breakdown by top-level model prefix
    print()
    prefix_counts: Dict[str, Counter] = defaultdict(Counter)
    for cls, rows in classified.items():
        for row in rows:
            prefix = row["model_rel_path"].split("/")[0]
            prefix_counts[prefix][cls] += 1

    for prefix, counter in prefix_counts.items():
        total_prefix = sum(counter.values())
        print(f"  - {prefix}: total={total_prefix} {dict(counter)}")

    # samples
    for cls in ("layering_diff", "ordering_diff", "other"):
        if cls not in classified:
            continue
        print(f"\n{cls} sample (lowest diff_count)")
        rows = sorted(classified[cls], key=lambda r: r["diff_count"])
        for row in rows[: args.limit]:
            print(
                f"  {row['model_rel_path']} diff={row['diff_count']} top={row['top_category']} first={row['first_diff']} phases={row['phase_signature']}"
            )

    print("\nPhase-root summary")
    for phase_root, model_count in phase_root_counter.most_common():
        total_diffs = phase_root_diff_counter[phase_root]
        avg_diff = total_diffs / model_count if model_count else 0.0
        print(
            f"  {phase_root}: models={model_count} total_diffs={total_diffs} avg_diff={avg_diff:.2f} "
            f"buckets={dict(phase_root_bucket_counter[phase_root])}"
        )

    out_models_tsv = Path(args.out_models_tsv)
    out_models_tsv.parent.mkdir(parents=True, exist_ok=True)
    with out_models_tsv.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t")
        writer.writerow(
            [
                "model_rel_path",
                "class",
                "phase_root",
                "phase_reason",
                "diff_bucket",
                "diff_count",
                "top_category",
                "first_diff",
                "phase_signature",
                "phase_symptoms",
            ]
        )
        for phase_root in sorted(
            phase_root_rows.keys(),
            key=lambda key: (
                -phase_root_counter[key],
                PHASE_ROOT_ORDER.index(key)
                if key in PHASE_ROOT_ORDER
                else len(PHASE_ROOT_ORDER),
                key,
            ),
        ):
            rows = sorted(
                phase_root_rows[phase_root],
                key=lambda row: (
                    DIFF_BUCKET_RANK[row["diff_bucket"]],
                    row["diff_count"],
                    row["model_rel_path"],
                ),
            )
            for row in rows:
                writer.writerow(
                    [
                        row["model_rel_path"],
                        row["class"],
                        row["phase_root"],
                        row["phase_reason"],
                        row["diff_bucket"],
                        row["diff_count"],
                        row["top_category"],
                        row["first_diff"],
                        row["phase_signature"],
                        counter_to_compact(row["phases"]),
                    ]
                )

    out_summary_md = Path(args.out_summary_md)
    out_summary_md.parent.mkdir(parents=True, exist_ok=True)
    with out_summary_md.open("w", encoding="utf-8") as handle:
        handle.write("# Drift Phase-Root Summary\n\n")
        handle.write(f"- Drift models: **{sum(phase_root_counter.values())}**\n")
        handle.write(f"- Source details: `{args.diff_details}`\n")
        handle.write(f"- Source manifest: `{args.manifest}`\n\n")
        handle.write(
            "| Phase Root | Models | Share | Total Diffs | Avg Diff | Low(1-5) | Medium(6-19) | High(20) | Top Prefixes | Top Categories |\n"
        )
        handle.write(
            "|---|---:|---:|---:|---:|---:|---:|---:|---|---|\n"
        )
        total_models = sum(phase_root_counter.values())
        for phase_root, model_count in phase_root_counter.most_common():
            total_diffs = phase_root_diff_counter[phase_root]
            avg_diff = total_diffs / model_count if model_count else 0.0
            share = (model_count / total_models * 100.0) if total_models else 0.0
            bucket_counts = phase_root_bucket_counter[phase_root]
            top_prefixes = ", ".join(
                f"{key}:{value}"
                for key, value in phase_root_prefix_counter[phase_root].most_common(3)
            )
            top_categories = ", ".join(
                f"{key}:{value}"
                for key, value in phase_root_category_counter[phase_root].most_common(3)
            )
            handle.write(
                f"| {phase_root} | {model_count} | {share:.1f}% | {total_diffs} | {avg_diff:.2f} | "
                f"{bucket_counts['low_1_5']} | {bucket_counts['medium_6_19']} | {bucket_counts['high_20_cap']} | "
                f"{top_prefixes} | {top_categories} |\n"
            )

    top_phase_roots = [
        phase for phase, _ in phase_root_counter.most_common(max(args.focus_top_phases, 0))
    ]
    focus_rows: List[dict] = []
    for phase_root in top_phase_roots:
        rows = sorted(
            phase_root_rows[phase_root],
            key=lambda row: (
                DIFF_BUCKET_RANK[row["diff_bucket"]],
                row["diff_count"],
                row["model_rel_path"],
            ),
        )
        focus_rows.extend(rows)

    out_focus_tsv = Path(args.out_focus_tsv)
    out_focus_tsv.parent.mkdir(parents=True, exist_ok=True)
    with out_focus_tsv.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t")
        writer.writerow(
            [
                "priority_rank",
                "phase_root",
                "diff_bucket",
                "diff_count",
                "model_rel_path",
                "class",
                "top_category",
                "first_diff",
            ]
        )
        for index, row in enumerate(focus_rows, start=1):
            writer.writerow(
                [
                    index,
                    row["phase_root"],
                    row["diff_bucket"],
                    row["diff_count"],
                    row["model_rel_path"],
                    row["class"],
                    row["top_category"],
                    row["first_diff"],
                ]
            )

    out_focus_md = Path(args.out_focus_md)
    out_focus_md.parent.mkdir(parents=True, exist_ok=True)
    with out_focus_md.open("w", encoding="utf-8") as handle:
        handle.write("# Top Phase Focus Queue\n\n")
        handle.write(f"- Top phase roots: {', '.join(top_phase_roots) if top_phase_roots else '(none)'}\n")
        handle.write(f"- Candidate models: **{len(focus_rows)}**\n\n")
        for phase_root in top_phase_roots:
            rows = sorted(
                phase_root_rows[phase_root],
                key=lambda row: (
                    DIFF_BUCKET_RANK[row["diff_bucket"]],
                    row["diff_count"],
                    row["model_rel_path"],
                ),
            )
            handle.write(f"## {phase_root}\n\n")
            handle.write(
                f"- models={phase_root_counter[phase_root]}, total_diffs={phase_root_diff_counter[phase_root]}, "
                f"buckets={dict(phase_root_bucket_counter[phase_root])}\n\n"
            )
            handle.write("| Priority | Bucket | Diffs | Model |\n")
            handle.write("|---:|---|---:|---|\n")
            for index, row in enumerate(rows[:30], start=1):
                handle.write(
                    f"| {index} | {row['diff_bucket']} | {row['diff_count']} | {row['model_rel_path']} |\n"
                )
            handle.write("\n")

    print(f"\nWrote phase-root model classification: {out_models_tsv}")
    print(f"Wrote phase-root summary: {out_summary_md}")
    print(f"Wrote top-phase focus queue: {out_focus_tsv}")
    print(f"Wrote top-phase focus summary: {out_focus_md}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
