#!/usr/bin/env python3
"""Compare Java and Rust ELK phase trace snapshots to pinpoint the first divergence step.

Usage (single model):
    python scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir> [options]

Usage (batch over all models):
    python scripts/compare_phase_traces.py <java_base> <rust_base> --batch

Each trace directory contains step JSON files named like:
    step_00_import.json
    step_01_EdgeAndLayerConstraintEdgeReverser.json
    step_02_PortSideProcessor.json
    ...

Each snapshot JSON has the structure:
    {
      "step": 0,
      "processor": "...",
      "nodes": [{"id": "N1", "x": 0.0, "y": 0.0, "width": 44.0, "height": 24.0,
                 "type": "NORMAL", "layer": 0,
                 "ports": [{"id": "P1", "x": 0.0, "y": 12.0, "side": "WEST",
                            "labels": []}],
                 "labels": [{"text": "...", "x": 5.0, "y": 5.0,
                             "width": 34.0, "height": 14.0}]}],
      "edges": [{"id": "E1", "sourceNode": "N1", "sourcePort": "P1",
                 "targetNode": "N2", "targetPort": "P2",
                 "bendPoints": [{"x": 10.0, "y": 20.0}], "labels": []}],
      "layers": [["N1", "N3"], ["N2"]],
      "graphSize": {"width": 100.0, "height": 80.0},
      "padding": {"top": 12.0, "bottom": 12.0, "left": 12.0, "right": 12.0}
    }
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
from pathlib import Path
from typing import Any, Iterator, Optional


# ---------------------------------------------------------------------------
# ANSI colour helpers (suppressed when not a tty or NO_COLOR is set)
# ---------------------------------------------------------------------------

_NO_COLOR: bool = (not sys.stdout.isatty()) or bool(os.environ.get("NO_COLOR"))


def _c(code: str, text: str) -> str:
    return text if _NO_COLOR else f"\033[{code}m{text}\033[0m"


def green(text: str) -> str:  return _c("32", text)
def red(text: str) -> str:    return _c("31", text)
def yellow(text: str) -> str: return _c("33", text)
def bold(text: str) -> str:   return _c("1",  text)


# ---------------------------------------------------------------------------
# JSON loading
# ---------------------------------------------------------------------------

def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as fh:
        result = json.load(fh)
        if not isinstance(result, dict): raise ValueError("snapshot must be a JSON object")
        return result


# ---------------------------------------------------------------------------
# Step-file discovery
# ---------------------------------------------------------------------------

def _step_index(filename: str) -> Optional[int]:
    """Extract numeric index from 'step_03_Foo.json'; return None if not a step file."""
    stem = Path(filename).stem
    if not stem.startswith("step_"):
        return None
    parts = stem.split("_", 2)
    try:
        return int(parts[1])
    except (IndexError, ValueError):
        return None


def _processor_from_stem(stem: str) -> str:
    """'step_15_LabelAndNodeSizeProcessor' -> 'LabelAndNodeSizeProcessor'."""
    parts = stem.split("_", 2)
    return parts[2] if len(parts) == 3 else (parts[1] if len(parts) == 2 else stem)


def _normalize_processor_name(name: str) -> str:
    """Normalize processor names across Java/Rust trace formats.

    Examples:
    - "PortSideProcessor" -> "PortSideProcessor"
    - "org::...::PortSideProcessor" -> "PortSideProcessor"
    - "org.eclipse....PortSideProcessor" -> "PortSideProcessor"
    """
    normalized = name.strip()
    if not normalized:
        return normalized
    if "::" in normalized:
        normalized = normalized.rsplit("::", 1)[1]
    if "." in normalized:
        normalized = normalized.rsplit(".", 1)[1]
    if "$" in normalized:
        normalized = normalized.rsplit("$", 1)[1]
    return normalized


def collect_step_files(directory: Path) -> list[tuple[int, str, Path]]:
    """Return sorted (index, stem, path) for all step_*.json files in *directory*."""
    results: list[tuple[int, str, Path]] = []
    for entry in directory.iterdir():
        if entry.suffix != ".json":
            continue
        idx = _step_index(entry.name)
        if idx is None:
            continue
        results.append((idx, entry.stem, entry))
    results.sort(key=lambda t: t[0])
    indices = [item[0] for item in results if item[0] >= 0]
    if len(indices) != len(set(indices)):
        raise ValueError(f"duplicate step indices in {directory}")
    if indices and indices != list(range(indices[-1] + 1)):
        raise ValueError(f"non-contiguous step indices in {directory}")
    return results


# ---------------------------------------------------------------------------
# Diff data structure
# ---------------------------------------------------------------------------

class DiffEntry:
    __slots__ = ("path", "java_val", "rust_val", "delta")

    def __init__(self, path: str, java_val: Any, rust_val: Any, delta: Optional[float] = None):
        self.path = path
        self.java_val = java_val
        self.rust_val = rust_val
        self.delta = delta

    # ------------------------------------------------------------------
    # Serialisation

    def to_dict(self) -> dict:
        d: dict = {"path": self.path, "java": self.java_val, "rust": self.rust_val}
        if self.delta is not None:
            d["delta"] = round(self.delta, 6)
        return d

    # ------------------------------------------------------------------
    # Human-readable one-liner

    def summary_line(self) -> str:
        if self.delta is not None:
            sign = "+" if self.delta >= 0 else ""
            return (f"  {self.path}: {self.java_val} vs {self.rust_val} "
                    f"(delta: {sign}{self.delta})")
        return f"  {self.path}: {self.java_val!r} vs {self.rust_val!r}"


# ---------------------------------------------------------------------------
# Snapshot comparator
# ---------------------------------------------------------------------------

class SnapshotComparator:
    """Compares two ELK phase snapshot dicts and returns a list of DiffEntry."""

    def __init__(self, tolerance: float = 0.001, max_diffs: int = 500):
        self.tol = tolerance
        self.max_diffs = max_diffs

    # ------------------------------------------------------------------
    # Within-layer node sorting for deterministic comparison

    @staticmethod
    def _node_sort_key(node: dict) -> tuple:
        """Composite key for deterministic within-layer node ordering.

        Uses properties that are identical between Java and Rust traces:
        label texts, node type, port count/sides, and geometry.
        """
        labels = tuple(l.get("text", "") for l in node.get("labels", []))
        ports = node.get("ports", [])
        port_sides = tuple(sorted(p.get("side", "") for p in ports))
        return (
            labels,
            node.get("type", ""),
            len(ports),
            port_sides,
            round(node.get("width", 0), 4),
            round(node.get("height", 0), 4),
            round(node.get("x", 0), 4),
            round(node.get("y", 0), 4),
        )

    @staticmethod
    def _sort_nodes_within_layers(all_nodes: list[dict]) -> list[dict]:
        """Sort nodes within each layer group for deterministic positional matching.

        Nodes are grouped by their ``layer`` field, sorted within each group
        by :meth:`_node_sort_key`, and reassembled in layer order (layerless
        nodes at the end).  This handles non-deterministic within-layer
        ordering from processors like NetworkSimplexLayerer.
        """
        from collections import defaultdict
        layer_groups: dict[int, list[dict]] = defaultdict(list)
        layerless: list[dict] = []
        for node in all_nodes:
            layer_idx = node.get("layer", -1)
            if isinstance(layer_idx, (int, float)) and layer_idx >= 0:
                layer_groups[int(layer_idx)].append(node)
            else:
                layerless.append(node)

        for idx in layer_groups:
            layer_groups[idx].sort(key=SnapshotComparator._node_sort_key)

        result: list[dict] = []
        for idx in sorted(layer_groups):
            result.extend(layer_groups[idx])
        result.extend(layerless)
        return result

    # ------------------------------------------------------------------
    # Normalization: convert Java/Rust formats to common representation

    @staticmethod
    def _normalize(snap: dict) -> dict:
        """Normalize a snapshot so both Java and Rust use the same structure.

        Handles two Java formats:
        - Flat: top-level ``nodes``/``edges``/``layers``
        - Component: ``components`` array, each with own nodes/edges/layers

        Common format after normalization:
        - ``all_nodes``: list of *all* node objects (layerless + in-layer)
        - ``layer_ids``: list[list[str]] – per-layer node-ID lists
        - ``edges``, ``graphSize``, ``padding``, ``offset``, ``size``
        """
        # If Java uses components format, flatten all components
        if "components" in snap and snap.get("components"):
            all_nodes: list[dict] = []
            layer_ids: list[list[str]] = []
            all_edges: list[dict] = []
            graph_size = None
            padding = None
            offset = None
            size = None
            for comp in snap["components"]:
                sub = SnapshotComparator._normalize(comp)
                all_nodes.extend(sub["all_nodes"])
                layer_ids.extend(sub["layer_ids"])
                all_edges.extend(sub["edges"])
                # Use first component's metadata as fallback
                if graph_size is None:
                    graph_size = sub.get("graphSize")
                if padding is None:
                    padding = sub.get("padding")
            return {
                "all_nodes": all_nodes,
                "layer_ids": layer_ids,
                "edges": all_edges,
                "graphSize": graph_size or snap.get("graphSize"),
                "padding": padding or snap.get("padding"),
                "offset": snap.get("offset"),
                "size": snap.get("size"),
            }

        all_nodes = list(snap.get("nodes", []))
        layer_ids: list[list[str]] = []

        for layer in snap.get("layers", []):
            if isinstance(layer, dict) and "nodes" in layer:
                # Rust format: {"index": i, "nodes": [{full node objects}]}
                layer_nodes = layer["nodes"]
                all_nodes.extend(layer_nodes)
                layer_ids.append([n["id"] for n in layer_nodes])
            elif isinstance(layer, list):
                # Java format: ["N1", "N3", ...]
                layer_ids.append(list(layer))
            else:
                layer_ids.append([])

        # Sort nodes within each layer for deterministic positional matching.
        # Some processors (e.g. NetworkSimplexLayerer) produce non-deterministic
        # within-layer ordering due to internal data-structure differences
        # (HashMap in Java vs Vec in Rust).  Sorting by node properties ensures
        # consistent matching regardless of implementation order.
        all_nodes = SnapshotComparator._sort_nodes_within_layers(all_nodes)
        all_nodes.extend(SnapshotComparator._sort_nodes_within_layers(snap.get("externalNodes", [])))
        # Port storage order is not geometric identity. Resolve coincident ports
        # by their incident nodes before assigning cross-language endpoint IDs.
        node_indices = {node.get("id"): index for index, node in enumerate(all_nodes)}
        incident = {}
        for edge in snap.get("edges", []):
            for side, other in (("source", "target"), ("target", "source")):
                incident.setdefault(edge.get(side + "Port"), []).append(
                    (side, node_indices.get(edge.get(other + "Node"), -1)))
        def port_key(port):
            shape = {key: value for key, value in port.items() if key != "id"}
            def numeric(value):
                if isinstance(value, (int, float)) and not isinstance(value, bool):
                    rounded = round(float(value), 4)
                    return 0.0 if rounded == 0 else rounded
                if isinstance(value, dict): return {key: numeric(item) for key, item in value.items()}
                if isinstance(value, list): return [numeric(item) for item in value]
                return value
            return (json.dumps(numeric(shape), sort_keys=True), sorted(incident.get(port.get("id"), [])))
        all_nodes = [dict(node, ports=sorted(node.get("ports", []), key=port_key)) for node in all_nodes]

        return {
            "all_nodes": all_nodes,
            "layer_ids": layer_ids,
            "edges": snap.get("edges", []),
            "graphSize": snap.get("graphSize"),
            "padding": snap.get("padding"),
            "offset": snap.get("offset"),
            "size": snap.get("size"),
        }

    # ------------------------------------------------------------------
    # Public entry point

    def compare(self, java: dict, rust: dict) -> list[DiffEntry]:
        diffs: list[DiffEntry] = []
        self._finite_tree(java, "java", diffs)
        self._finite_tree(rust, "rust", diffs)
        if diffs:
            return diffs
        if "components" in java or "components" in rust:
            jc = java.get("components", [java])
            rc = rust.get("components", [rust])
            if len(jc) != len(rc):
                self._push(diffs, "components/count", len(jc), len(rc))
            for index, (jcomp, rcomp) in enumerate(zip(jc, rc)):
                for diff in self.compare(jcomp, rcomp):
                    diff.path = f"components[{index}]/" + diff.path
                    diffs.append(diff)
            return diffs[:self.max_diffs]
        jn = self._normalize(java)
        rn = self._normalize(rust)
        self._all_nodes(jn, rn, diffs)
        self._edges_norm(jn, rn, diffs)
        self._layer_structure(jn, rn, diffs)
        self._graph_size_norm(jn, rn, diffs)
        self._padding_norm(jn, rn, diffs)
        for key, fields in (("size", ("width", "height")), ("offset", ("x", "y"))):
            self._fields(jn.get(key), rn.get(key), fields, key, diffs)
        return diffs

    def _finite_tree(self, value, path, diffs):
        if isinstance(value, dict):
            for key, item in value.items():
                self._finite_tree(item, f"{path}/{key}", diffs)
        elif isinstance(value, list):
            for index, item in enumerate(value):
                self._finite_tree(item, f"{path}[{index}]", diffs)
        elif isinstance(value, float) and not math.isfinite(value):
            self._push(diffs, path, "finite number required", str(value))

    def _fields(self, left, right, fields, path, diffs):
        if left is None and right is None:
            return
        if not isinstance(left, dict) or not isinstance(right, dict):
            self._push(diffs, path, left, right)
            return
        for field in fields:
            self._num(left, right, field, path, diffs)

    # ------------------------------------------------------------------
    # All nodes (from normalized snapshot)

    def _all_nodes(self, jn: dict, rn: dict, diffs: list[DiffEntry]) -> None:
        jnodes = jn["all_nodes"]
        rnodes = rn["all_nodes"]
        # Always use positional matching (Java/Rust have different ID schemes: N1 vs N0)
        if len(jnodes) != len(rnodes):
            self._push(diffs, "nodes", f"count={len(jnodes)}", f"count={len(rnodes)}")
        for i in range(min(len(jnodes), len(rnodes))):
            if len(diffs) >= self.max_diffs:
                return
            jnode, rnode = jnodes[i], rnodes[i]
            self._node(jnode, rnode, f"nodes[{i}]", diffs)

    def _node(self, jn: dict, rn: dict, prefix: str, diffs: list[DiffEntry]) -> None:
        for f in ("x", "y", "width", "height"):
            self._num(jn, rn, f, prefix, diffs)
        for f in ("type", "layer"):
            self._scalar(jn, rn, f, prefix, diffs)
        for key in ("margin", "padding"):
            self._fields(jn.get(key), rn.get(key), ("top", "bottom", "left", "right"), f"{prefix}/{key}", diffs)
        # ports — positional matching (IDs differ between Java/Rust)
        jports = jn.get("ports", [])
        rports = rn.get("ports", [])
        if len(jports) != len(rports):
            self._push(diffs, f"{prefix}/ports", f"count={len(jports)}", f"count={len(rports)}")
        for i in range(min(len(jports), len(rports))):
            self._port(jports[i], rports[i], f"{prefix}/ports[{i}]", diffs)
        # labels
        self._labels(jn.get("labels", []), rn.get("labels", []), f"{prefix}/labels", diffs)

    def _port(self, jp: dict, rp: dict, prefix: str, diffs: list[DiffEntry]) -> None:
        for f in ("x", "y", "width", "height"):
            self._num(jp, rp, f, prefix, diffs)
        self._fields(jp.get("anchor"), rp.get("anchor"), ("x", "y"), f"{prefix}/anchor", diffs)
        self._fields(jp.get("margin"), rp.get("margin"), ("top", "bottom", "left", "right"), f"{prefix}/margin", diffs)
        self._scalar(jp, rp, "side", prefix, diffs)
        self._labels(jp.get("labels", []), rp.get("labels", []), f"{prefix}/labels", diffs)

    def _labels(self, jlist: list, rlist: list, prefix: str, diffs: list[DiffEntry]) -> None:
        if len(jlist) != len(rlist):
            self._push(diffs, prefix, f"count={len(jlist)}", f"count={len(rlist)}")
        for i in range(min(len(jlist), len(rlist))):
            lp = f"{prefix}[{i}]"
            for f in ("x", "y", "width", "height"):
                self._num(jlist[i], rlist[i], f, lp, diffs)
            self._scalar(jlist[i], rlist[i], "text", lp, diffs)

    # ------------------------------------------------------------------
    # Edges (normalized)

    def _edges_norm(self, jn: dict, rn: dict, diffs: list[DiffEntry]) -> None:
        jedges = self._canonical_edges(jn)
        redges = self._canonical_edges(rn)
        # Positional matching (IDs differ between Java/Rust)
        if len(jedges) != len(redges):
            self._push(diffs, "edges", f"count={len(jedges)}", f"count={len(redges)}")
        for i in range(min(len(jedges), len(redges))):
            if len(diffs) >= self.max_diffs:
                return
            self._edge(jedges[i], redges[i], f"edges[{i}]", diffs)

    @staticmethod
    def _canonical_edges(snap):
        nodes, ports = {}, {}
        for index, node in enumerate(snap["all_nodes"]):
            key = node.get("id")
            if key in nodes:
                raise ValueError(f"duplicate node ID: {key}")
            nodes[key] = index
            for port_index, port in enumerate(node.get("ports", [])):
                key = port.get("id")
                if key in ports:
                    raise ValueError(f"duplicate port ID: {key}")
                ports[key] = (index, port_index)
        edges = []
        for edge in snap["edges"]:
            item = dict(edge)
            for side in ("source", "target"):
                node = edge.get(side + "Node")
                port = edge.get(side + "Port")
                if node not in nodes or port not in ports or ports[port][0] != nodes[node]:
                    raise ValueError(f"invalid {side} endpoint: {node}/{port}")
                item[side + "Node"] = nodes[node]
                item[side + "Port"] = ports[port]
            edges.append(item)
        return sorted(edges, key=lambda e: (e["sourcePort"], e["targetPort"], json.dumps(e.get("bendPoints", []), sort_keys=True)))

    def _edge(self, je: dict, re_: dict, prefix: str, diffs: list[DiffEntry]) -> None:
        for field in ("sourceNode", "sourcePort", "targetNode", "targetPort"):
            self._scalar(je, re_, field, prefix, diffs)
        jbp = je.get("bendPoints", [])
        rbp = re_.get("bendPoints", [])
        if len(jbp) != len(rbp):
            self._push(diffs, f"{prefix}/bendPoints", f"count={len(jbp)}", f"count={len(rbp)}")
        for i in range(min(len(jbp), len(rbp))):
            bp = f"{prefix}/bendPoints[{i}]"
            self._num(jbp[i], rbp[i], "x", bp, diffs)
            self._num(jbp[i], rbp[i], "y", bp, diffs)
        self._labels(je.get("labels", []), re_.get("labels", []), f"{prefix}/labels", diffs)

    # ------------------------------------------------------------------
    # Layer membership after cross-language node identity mapping

    def _layer_structure(self, jn: dict, rn: dict, diffs: list[DiffEntry]) -> None:
        jl = jn["layer_ids"]
        rl = rn["layer_ids"]
        if len(jl) != len(rl):
            self._push(diffs, "layers", f"count={len(jl)}", f"count={len(rl)}")
        jids = {node.get("id"): i for i, node in enumerate(jn["all_nodes"])}
        rids = {node.get("id"): i for i, node in enumerate(rn["all_nodes"])}
        for i in range(min(len(jl), len(rl))):
            left = sorted(jids[node] for node in jl[i])
            right = sorted(rids[node] for node in rl[i])
            if left != right:
                self._push(diffs, f"layers[{i}]/members", left, right)

    def _graph_size_norm(self, jn: dict, rn: dict, diffs: list[DiffEntry]) -> None:
        jg = jn.get("graphSize")
        rg = rn.get("graphSize")
        if jg is None and rg is None:
            return
        if jg is None or rg is None:
            self._push(diffs, "graphSize", jg, rg)
            return
        for f in ("width", "height"):
            self._num(jg, rg, f, "graphSize", diffs)

    def _padding_norm(self, jn: dict, rn: dict, diffs: list[DiffEntry]) -> None:
        jp = jn.get("padding")
        rp = rn.get("padding")
        if jp is None and rp is None:
            return
        if jp is None or rp is None:
            self._push(diffs, "padding", jp, rp)
            return
        for f in ("top", "bottom", "left", "right"):
            self._num(jp, rp, f, "padding", diffs)

    # ------------------------------------------------------------------
    # Low-level helpers

    def _num(self, jd: dict, rd: dict, field: str, prefix: str, diffs: list[DiffEntry]) -> None:
        if len(diffs) >= self.max_diffs:
            return
        jv = jd.get(field)
        rv = rd.get(field)
        if jv is None and rv is None:
            return
        if jv is None or rv is None:
            self._push(diffs, f"{prefix}/{field}", jv, rv)
            return
        if isinstance(jv, (int, float)) and isinstance(rv, (int, float)):
            # Non-finite geometry is invalid even when both sides contain it.
            # Zero coordinates are compared normally; no numeric waivers apply.
            if not (math.isfinite(jv) and math.isfinite(rv)):
                self._push(diffs, f"{prefix}/{field}", jv, rv)
                return
            delta = float(jv) - float(rv)
            if abs(delta) > self.tol:
                diffs.append(DiffEntry(f"{prefix}/{field}", float(jv), float(rv), delta))
        elif jv != rv:
            self._push(diffs, f"{prefix}/{field}", jv, rv)

    def _scalar(self, jd: dict, rd: dict, field: str, prefix: str, diffs: list[DiffEntry]) -> None:
        if len(diffs) >= self.max_diffs:
            return
        jv = jd.get(field)
        rv = rd.get(field)
        if jv != rv:
            self._push(diffs, f"{prefix}/{field}", jv, rv)

    def _push(self, diffs: list[DiffEntry], path: str, jv: Any, rv: Any) -> None:
        if len(diffs) < self.max_diffs:
            diffs.append(DiffEntry(path, jv, rv))


# ---------------------------------------------------------------------------
# Per-step result
# ---------------------------------------------------------------------------

class StepResult:
    __slots__ = ("step_index", "processor", "status", "diffs", "cascaded")

    def __init__(self, step_index: Optional[int], processor: str, status: str, diffs: list[DiffEntry]):
        self.step_index = step_index   # None for unmatched steps
        self.processor = processor
        self.status = status           # "match" | "drift" | "missing_java" | "missing_rust"
        self.diffs = diffs
        self.cascaded = False          # set True after the first divergence

    def to_dict(self) -> dict:
        d: dict = {
            "step": self.step_index,
            "processor": self.processor,
            "status": self.status,
            "diffs": len(self.diffs),
        }
        if self.diffs:
            d["details"] = [e.to_dict() for e in self.diffs]
        if self.cascaded:
            d["cascaded"] = True
        return d


# ---------------------------------------------------------------------------
# Model comparison result
# ---------------------------------------------------------------------------

class ModelResult:
    def __init__(self, model: str, java_step_count: int, rust_step_count: int,
                 step_results: list[StepResult]):
        self.model = model
        self.java_step_count = java_step_count
        self.rust_step_count = rust_step_count
        self.steps = step_results
        self.first_div_step: Optional[int] = None
        self.first_div_processor: Optional[str] = None
        self._mark_cascades()

    def _mark_cascades(self) -> None:
        for sr in self.steps:
            if sr.status in ("drift", "missing_java", "missing_rust"):
                if self.first_div_step is None:
                    self.first_div_step = sr.step_index
                    self.first_div_processor = sr.processor
                else:
                    sr.cascaded = True

    @property
    def match_count(self) -> int:
        return sum(1 for s in self.steps if s.status == "match")

    @property
    def drift_count(self) -> int:
        return sum(1 for s in self.steps if s.status != "match")

    @property
    def total_diffs(self) -> int:
        return sum(len(s.diffs) for s in self.steps)

    @property
    def first_div_diffs(self) -> int:
        for s in self.steps:
            if s.step_index == self.first_div_step and not s.cascaded:
                return len(s.diffs)
        return 0

    def to_dict(self) -> dict:
        return {
            "model": self.model,
            "java_steps": self.java_step_count,
            "rust_steps": self.rust_step_count,
            "first_divergence_step": self.first_div_step,
            "first_divergence_processor": self.first_div_processor,
            "steps": [s.to_dict() for s in self.steps],
            "summary": {
                "match": self.match_count,
                "drift": self.drift_count,
                "total_diffs": self.total_diffs,
            },
        }


def validate_snapshot(snapshot: dict, side: str) -> list[DiffEntry]:
    """Reject malformed or null geometry even when both exports contain it."""
    errors = []
    def require(condition, path):
        if not condition: errors.append(DiffEntry(side + "/" + path, "valid trace field required", None))
    def numbers(value, fields, path):
        if not isinstance(value, dict):
            require(False, path); return
        for field in fields:
            number = value.get(field)
            require(isinstance(number, (int, float)) and not isinstance(number, bool) and math.isfinite(number), path + "/" + field)
    def labels(values, path):
        require(isinstance(values, list), path)
        if not isinstance(values, list): return
        for i, label in enumerate(values):
            numbers(label, ("x", "y", "width", "height"), f"{path}[{i}]")
    def node(value, path):
        numbers(value, ("x", "y", "width", "height", "layer"), path)
        if not isinstance(value, dict): return
        require(isinstance(value.get("id"), str), path + "/id")
        require(isinstance(value.get("type"), str), path + "/type")
        for box in ("margin", "padding"):
            numbers(value.get(box), ("top", "bottom", "left", "right"), path + "/" + box)
        labels(value.get("labels"), path + "/labels")
        require(isinstance(value.get("ports"), list), path + "/ports")
        for i, port in enumerate(value.get("ports", [])):
            pp = f"{path}/ports[{i}]"
            numbers(port, ("x", "y", "width", "height"), pp)
            if not isinstance(port, dict): continue
            require(isinstance(port.get("id"), str) and isinstance(port.get("side"), str), pp + "/id-side")
            numbers(port.get("anchor"), ("x", "y"), pp + "/anchor")
            numbers(port.get("margin"), ("top", "bottom", "left", "right"), pp + "/margin")
            labels(port.get("labels"), pp + "/labels")
    if "components" in snapshot:
        components = snapshot["components"]
        require(isinstance(components, list) and bool(components), "components")
        if not isinstance(components, list):
            return errors
        for i, component in enumerate(components):
            if not isinstance(component, dict):
                require(False, f"components[{i}]")
                continue
            errors.extend(validate_snapshot(component, f"{side}/components[{i}]"))
        return errors
    for key, fields in (("size", ("width", "height")), ("graphSize", ("width", "height")), ("offset", ("x", "y")), ("padding", ("top", "bottom", "left", "right"))):
        numbers(snapshot.get(key), fields, key)
    for key in ("nodes", "layers", "edges"):
        require(isinstance(snapshot.get(key), list), key)
    for i, value in enumerate(snapshot.get("nodes", [])): node(value, f"nodes[{i}]")
    for i, value in enumerate(snapshot.get("externalNodes", [])): node(value, f"externalNodes[{i}]")
    for i, layer in enumerate(snapshot.get("layers", [])):
        if isinstance(layer, dict):
            require(layer.get("index") == i, f"layers[{i}]/index")
            for j, value in enumerate(layer.get("nodes", [])): node(value, f"layers[{i}]/nodes[{j}]")
        else: require(isinstance(layer, list), f"layers[{i}]")
    for i, edge in enumerate(snapshot.get("edges", [])):
        for key in ("sourceNode", "sourcePort", "targetNode", "targetPort"):
            require(isinstance(edge.get(key), str), f"edges[{i}]/{key}")
        require(isinstance(edge.get("bendPoints"), list), f"edges[{i}]/bendPoints")
        for j, point in enumerate(edge.get("bendPoints", [])): numbers(point, ("x", "y"), f"edges[{i}]/bendPoints[{j}]")
        labels(edge.get("labels"), f"edges[{i}]/labels")
    return errors


# ---------------------------------------------------------------------------
# Core comparison function
# ---------------------------------------------------------------------------

def compare_model(
    java_dir: Path,
    rust_dir: Path,
    model: str,
    comparator: SnapshotComparator,
    stop_at_first: bool = False,
) -> ModelResult:
    java_steps = collect_step_files(java_dir)
    rust_steps = collect_step_files(rust_dir)

    # Skip step_-1_INITIAL (Java-only artifact, no Rust equivalent)
    # Keep original step ordering strict: component ordering differences are treated as drift.
    jsteps = [(i, s, p) for i, s, p in java_steps if i >= 0]
    rsteps = [(i, s, p) for i, s, p in rust_steps if i >= 0]
    jmap: dict[int, tuple[str, Path]] = {i: (s, p) for i, s, p in jsteps}
    rmap: dict[int, tuple[str, Path]] = {i: (s, p) for i, s, p in rsteps}

    all_indices = sorted(set(jmap) | set(rmap))
    step_results: list[StepResult] = []
    diverged = False
    if not all_indices:
        return ModelResult(model, len(java_steps), len(rust_steps), [StepResult(0, "empty", "drift", [DiffEntry("steps", "nonempty trace required", "empty")])])

    for idx in all_indices:
        in_java = idx in jmap
        in_rust = idx in rmap

        if not in_java:
            proc = _processor_from_stem(rmap[idx][0])
            sr = StepResult(idx, proc, "missing_java", [])
            step_results.append(sr)
            if not diverged:
                diverged = True
                if stop_at_first:
                    break
            continue

        if not in_rust:
            proc = _processor_from_stem(jmap[idx][0])
            sr = StepResult(idx, proc, "missing_rust", [])
            step_results.append(sr)
            if not diverged:
                diverged = True
                if stop_at_first:
                    break
            continue

        j_stem, j_path = jmap[idx]
        r_stem, r_path = rmap[idx]
        proc = _processor_from_stem(j_stem)

        try:
            j_snap = load_json(j_path)
        except Exception as exc:
            sr = StepResult(idx, proc, "missing_java",
                            [DiffEntry(f"step_{idx:02d}/load_error", str(exc), None)])
            step_results.append(sr)
            if not diverged:
                diverged = True
                if stop_at_first:
                    break
            continue

        try:
            r_snap = load_json(r_path)
        except Exception as exc:
            sr = StepResult(idx, proc, "missing_rust",
                            [DiffEntry(f"step_{idx:02d}/load_error", None, str(exc))])
            step_results.append(sr)
            if not diverged:
                diverged = True
                if stop_at_first:
                    break
            continue

        j_proc_raw = j_snap.get("processor") or _processor_from_stem(j_stem)
        r_proc_raw = r_snap.get("processor") or _processor_from_stem(r_stem)
        j_proc = _normalize_processor_name(str(j_proc_raw))
        r_proc = _normalize_processor_name(str(r_proc_raw))
        proc = j_proc or proc

        diffs: list[DiffEntry] = []
        if j_proc != r_proc:
            diffs.append(DiffEntry("processor", j_proc, r_proc))
        for side, snap, stem in (("java", j_snap, j_stem), ("rust", r_snap, r_stem)):
            if snap.get("step") != idx or _normalize_processor_name(str(snap.get("processor", ""))) != _normalize_processor_name(_processor_from_stem(stem)):
                diffs.append(DiffEntry(side + "/step_metadata", idx, snap.get("step")))
            for field in ("nodes", "edges", "layers", "graphSize", "size", "offset", "padding"):
                if field not in snap and "components" not in snap:
                    diffs.append(DiffEntry(side + "/" + field, "required field", None))
        schema_diffs = validate_snapshot(j_snap, "java") + validate_snapshot(r_snap, "rust")
        state_diffs = schema_diffs or comparator.compare(j_snap, r_snap)
        if state_diffs:
            remaining = max(comparator.max_diffs - len(diffs), 0)
            diffs.extend(state_diffs[:remaining])

        if diffs:
            sr = StepResult(idx, proc, "drift", diffs)
            step_results.append(sr)
            if not diverged:
                diverged = True
                if stop_at_first:
                    break
        else:
            step_results.append(StepResult(idx, proc, "match", []))

    return ModelResult(model, len(java_steps), len(rust_steps), step_results)


# ---------------------------------------------------------------------------
# Output: single model (human-readable)
# ---------------------------------------------------------------------------

def _status_tag(sr: StepResult) -> str:
    if sr.status == "match":
        return green("MATCH")
    if sr.status == "missing_java":
        return yellow("MISSING (java only)")
    if sr.status == "missing_rust":
        return yellow("MISSING (rust only)")
    tag = f"DRIFT ({len(sr.diffs)} diffs"
    if sr.cascaded:
        tag += ", cascaded from earlier step"
    tag += ")"
    return red(tag)


def print_model_result(result: ModelResult, verbose: bool) -> None:
    model_label = result.model or "(model)"
    print(f"=== Phase Trace Comparison: {model_label} ===")

    if result.java_step_count != result.rust_step_count:
        print(f"Java steps: {result.java_step_count}, "
              f"Rust steps: {result.rust_step_count}  "
              f"{yellow('[STEP COUNT MISMATCH]')}")
    else:
        print(f"Java steps: {result.java_step_count}, Rust steps: {result.rust_step_count}")
    print()

    max_proc = max((len(s.processor) for s in result.steps), default=10)
    col = max(max_proc + 4, 50)

    for sr in result.steps:
        idx_str = f"{sr.step_index:02d}" if sr.step_index is not None else "??"
        label = f"Step {idx_str} ({sr.processor}):"
        is_first = (
            sr.step_index == result.first_div_step
            and not sr.cascaded
            and sr.status != "match"
        )
        marker = bold("  <- FIRST DIVERGENCE") if is_first else ""
        print(f"{label:<{col}} {_status_tag(sr)}{marker}")

        show_diffs = verbose or is_first
        if show_diffs and sr.diffs:
            for de in sr.diffs[:20]:
                print(de.summary_line())
            if len(sr.diffs) > 20:
                print(f"    ... and {len(sr.diffs) - 20} more diffs")

    print()
    if result.first_div_step is not None:
        n = result.first_div_diffs
        print(bold(
            f"Summary: first divergence at step {result.first_div_step:02d} "
            f"({result.first_div_processor}), {n} diffs"
        ))
    else:
        print(green("Summary: no divergence detected"))

    print(f"Total: {result.match_count} MATCH, {result.drift_count} DRIFT")


# ---------------------------------------------------------------------------
# Output: batch (human-readable table)
# ---------------------------------------------------------------------------

def print_batch_results(
    results: list[ModelResult],
    java_only: Optional[set] = None,
    rust_only: Optional[set] = None,
    report_missing: bool = False,
) -> None:
    col_m = max((len(r.model) for r in results), default=5)
    col_m = max(col_m, 24)
    col_p = max((len(r.first_div_processor or "-") for r in results), default=10)
    col_p = max(col_p, 24)

    header = (f"{'Model':<{col_m}}  {'Steps':>5}  "
              f"{'FirstDiv':>8}  {'Processor':<{col_p}}  {'Diffs':>5}")
    print(bold(header))
    print("-" * len(header))

    for r in results:
        first_div = str(r.first_div_step) if r.first_div_step is not None else "-"
        proc = r.first_div_processor or "-"
        diffs_str = str(r.first_div_diffs) if r.first_div_step is not None else "-"
        steps = max(r.java_step_count, r.rust_step_count)
        model_str = (green if r.first_div_step is None else red)(r.model)
        print(f"{model_str:<{col_m + 9}}  {steps:>5}  "
              f"{first_div:>8}  {proc:<{col_p}}  {diffs_str:>5}")

    print()
    n_match = sum(1 for r in results if r.first_div_step is None)
    n_drift = len(results) - n_match
    n_java_only = len(java_only) if java_only else 0
    n_rust_only = len(rust_only) if rust_only else 0
    summary = (f"Batch summary: {n_match} match, {n_drift} drift out of {len(results)} models"
               f" (java_only={n_java_only}, rust_only={n_rust_only})")
    print(bold(summary))

    if report_missing and (n_java_only > 0 or n_rust_only > 0):
        print()
        if java_only:
            print(yellow(f"Java-only models ({n_java_only}):"))
            for name in sorted(java_only)[:50]:
                print(f"  {name}")
            if n_java_only > 50:
                print(f"  ... and {n_java_only - 50} more")
        if rust_only:
            print(yellow(f"Rust-only models ({n_rust_only}):"))
            for name in sorted(rust_only)[:50]:
                print(f"  {name}")
            if n_rust_only > 50:
                print(f"  ... and {n_rust_only - 50} more")


# ---------------------------------------------------------------------------
# Batch-mode directory detection and iteration
# ---------------------------------------------------------------------------

def _has_step_files(directory: Path) -> bool:
    for entry in directory.iterdir():
        if entry.suffix == ".json" and _step_index(entry.name) is not None:
            return True
    return False


def _is_batch_dir(java_base: Path, rust_base: Path) -> bool:
    """True if the dirs hold per-model sub-dirs rather than step files directly."""
    for entry in java_base.iterdir():
        if entry.is_dir() and _has_step_files(entry):
            return True
    return False


def _find_trace_dirs(base: Path) -> dict[str, Path]:
    """Recursively find directories containing step files, keyed by normalized relative path."""
    result: dict[str, Path] = {}
    for dirpath, _dirnames, filenames in os.walk(base):
        if any(f.startswith("step_") and f.endswith(".json") for f in filenames):
            rel = os.path.relpath(dirpath, base)
            # Normalize: strip trailing .json from directory names
            normalized = rel.removesuffix(".json")
            if normalized in result:
                raise ValueError(f"duplicate normalized model path: {normalized}")
            result[normalized] = Path(dirpath)
    return result


def iter_model_pairs(
    java_base: Path, rust_base: Path
) -> tuple[Iterator[tuple[str, Path, Path]], set[str], set[str]]:
    """Return (iterator of common pairs, java_only names, rust_only names)."""
    jmodels = _find_trace_dirs(java_base)
    rmodels = _find_trace_dirs(rust_base)
    common = sorted(set(jmodels) & set(rmodels))
    java_only = set(jmodels) - set(rmodels)
    rust_only = set(rmodels) - set(jmodels)

    def _iter() -> Iterator[tuple[str, Path, Path]]:
        for name in common:
            yield name, jmodels[name], rmodels[name]

    return _iter(), java_only, rust_only


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="compare_phase_traces.py",
        description="Compare Java and Rust ELK phase trace snapshots to find the first divergence step.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Single model
  python scripts/compare_phase_traces.py \\
      tests/model_parity/java/trace/verticalOrder/ \\
      tests/model_parity/rust/trace/verticalOrder/

  # Single model, verbose diff output
  python scripts/compare_phase_traces.py \\
      tests/model_parity/java/trace/verticalOrder/ \\
      tests/model_parity/rust/trace/verticalOrder/ \\
      --verbose

  # Stop at first divergence
  python scripts/compare_phase_traces.py \\
      tests/model_parity/java/trace/verticalOrder/ \\
      tests/model_parity/rust/trace/verticalOrder/ \\
      --stop-at-first

  # JSON output
  python scripts/compare_phase_traces.py \\
      tests/model_parity/java/trace/verticalOrder/ \\
      tests/model_parity/rust/trace/verticalOrder/ \\
      --json

  # Batch over all models
  python scripts/compare_phase_traces.py \\
      tests/model_parity/java/trace/ \\
      tests/model_parity/rust/trace/ \\
      --batch
""",
    )
    parser.add_argument("java_trace_dir", type=Path,
                        help="Java trace directory (or base dir for --batch)")
    parser.add_argument("rust_trace_dir", type=Path,
                        help="Rust trace directory (or base dir for --batch)")
    parser.add_argument("--tolerance", "-t", type=float, default=0.001, metavar="FLOAT",
                        help="Numeric comparison tolerance (default: 0.001)")
    parser.add_argument("--verbose", "-v", action="store_true",
                        help="Show full diff details for every mismatched step")
    parser.add_argument("--stop-at-first", action="store_true",
                        help="Stop processing at the first step with differences")
    parser.add_argument("--json", dest="output_json", action="store_true",
                        help="Output results as JSON instead of human-readable text")
    parser.add_argument("--batch", "-b", action="store_true",
                        help="Treat args as base dirs containing per-model sub-dirs "
                             "(auto-detected if sub-dirs contain step files)")
    parser.add_argument("--max-diffs", type=int, default=500, metavar="N",
                        help="Maximum diffs collected per step (default: 500)")
    parser.add_argument("--report-missing", action="store_true",
                        help="Report models present in only one trace directory (batch mode)")
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    java_base: Path = args.java_trace_dir
    rust_base: Path = args.rust_trace_dir

    if not java_base.exists():
        print(f"error: java_trace_dir does not exist: {java_base}", file=sys.stderr)
        return 2
    if not rust_base.exists():
        print(f"error: rust_trace_dir does not exist: {rust_base}", file=sys.stderr)
        return 2

    if not math.isfinite(args.tolerance) or args.tolerance < 0 or args.max_diffs < 1:
        parser.error("tolerance must be finite and nonnegative; max-diffs must be positive")
    comparator = SnapshotComparator(tolerance=args.tolerance, max_diffs=args.max_diffs)
    batch_mode = args.batch or _is_batch_dir(java_base, rust_base)

    # ------------------------------------------------------------------
    # Batch mode
    if batch_mode:
        pairs_iter, java_only, rust_only = iter_model_pairs(java_base, rust_base)
        results: list[ModelResult] = []
        for model_name, jdir, rdir in pairs_iter:
            r = compare_model(jdir, rdir, model_name, comparator,
                              stop_at_first=args.stop_at_first)
            results.append(r)

        if args.output_json:
            out = {
                "batch": True,
                "models": [r.to_dict() for r in results],
                "summary": {
                    "total_models": len(results),
                    "all_match": sum(1 for r in results if r.first_div_step is None),
                    "diverged": sum(1 for r in results if r.first_div_step is not None),
                    "java_only": len(java_only),
                    "rust_only": len(rust_only),
                },
            }
            print(json.dumps(out, indent=2))
        else:
            print_batch_results(results, java_only=java_only, rust_only=rust_only,
                                report_missing=args.report_missing)

        return 0 if results and not java_only and not rust_only and all(r.drift_count == 0 for r in results) else 1

    # ------------------------------------------------------------------
    # Single-model mode
    model_name = java_base.name
    result = compare_model(java_base, rust_base, model_name, comparator,
                           stop_at_first=args.stop_at_first)

    if args.output_json:
        print(json.dumps(result.to_dict(), indent=2))
    else:
        print_model_result(result, verbose=args.verbose)

    return 0 if result.first_div_step is None else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, TypeError, KeyError) as exc:
        print(f"error: invalid or incomplete trace: {exc}", file=sys.stderr)
        sys.exit(2)
