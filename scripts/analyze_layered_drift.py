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


def get_direction(input_json: dict) -> str:
    return input_json.get("layoutOptions", {}).get("org.eclipse.elk.direction", "RIGHT")


def layer_axis(direction: str) -> Tuple[str, str]:
    direction = direction.upper()
    if direction in ("RIGHT", "LEFT"):
        return "x", "y"
    return "y", "x"


def layer_index_by_coord(nodes: Iterable[dict], primary_key: str, tol: float = 1e-6):
    coords = sorted({n.get(primary_key) for n in nodes if primary_key in n})
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

    return uniq, idx


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

    _, java_idx = layer_index_by_coord((java_map[n] for n in common_ids), primary)
    _, rust_idx = layer_index_by_coord((rust_map[n] for n in common_ids), primary)

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
        "--prefix",
        action="append",
        default=[],
        help="Optional model path prefix filter (can be specified multiple times)",
    )
    args = parser.parse_args()

    manifest = load_manifest(Path(args.manifest))
    drift_rows = load_drift_rows(Path(args.diff_details))

    if args.prefix:
        drift_rows = [r for r in drift_rows if any(r["model_rel_path"].startswith(p) for p in args.prefix)]

    classified: Dict[str, List[dict]] = defaultdict(list)
    for row in drift_rows:
        manifest_row = manifest.get(row["model_rel_path"])
        if not manifest_row:
            continue
        cls = classify_model(
            Path(manifest_row["input_json"]),
            Path(manifest_row["java_layout_json"]),
            Path(manifest_row["rust_layout_json"]),
        )
        classified[cls].append(row)

    total = sum(len(rows) for rows in classified.values())
    print(f"Drift models classified: {total}")
    counts = Counter({cls: len(rows) for cls, rows in classified.items()})
    for cls, count in counts.most_common():
        print(f"  {cls}: {count}")

    # Breakdown by high-level prefix
    prefix_counts: Dict[str, Counter] = defaultdict(Counter)
    for cls, rows in classified.items():
        for row in rows:
            prefix = row["model_rel_path"].split("/")[0]
            prefix_counts[prefix][cls] += 1
    for prefix, counter in prefix_counts.items():
        total_prefix = sum(counter.values())
        print(f"  - {prefix}: total={total_prefix} {dict(counter)}")

    # Samples
    for cls in ("layering_diff", "ordering_diff", "other"):
        if cls not in classified:
            continue
        print(f"\n{cls} sample (lowest diff_count)")
        rows = sorted(classified[cls], key=lambda r: r["diff_count"])
        for row in rows[: args.limit]:
            print(
                f"  {row['model_rel_path']} diff={row['diff_count']} top={row['top_category']} first={row['first_diff']}"
            )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
