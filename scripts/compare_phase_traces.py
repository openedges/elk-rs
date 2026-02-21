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
        return json.load(fh)


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
        jn = self._normalize(java)
        rn = self._normalize(rust)
        self._all_nodes(jn, rn, diffs)
        self._edges_norm(jn, rn, diffs)
        self._layer_structure(jn, rn, diffs)
        self._graph_size_norm(jn, rn, diffs)
        self._padding_norm(jn, rn, diffs)
        return diffs

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
        for f in ("x", "y"):
            self._num(jp, rp, f, prefix, diffs)
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
        jedges = jn["edges"]
        redges = rn["edges"]
        # Positional matching (IDs differ between Java/Rust)
        if len(jedges) != len(redges):
            self._push(diffs, "edges", f"count={len(jedges)}", f"count={len(redges)}")
        for i in range(min(len(jedges), len(redges))):
            if len(diffs) >= self.max_diffs:
                return
            self._edge(jedges[i], redges[i], f"edges[{i}]", diffs)

    def _edge(self, je: dict, re_: dict, prefix: str, diffs: list[DiffEntry]) -> None:
        # Skip source/target ID fields — Java and Rust use different ID schemes
        # (Java: "source"/"target" with ElkNode IDs, Rust: "source"/"target" with N{id})
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
    # Layer structure (normalized — just check counts, not IDs)

    def _layer_structure(self, jn: dict, rn: dict, diffs: list[DiffEntry]) -> None:
        jl = jn["layer_ids"]
        rl = rn["layer_ids"]
        if len(jl) != len(rl):
            self._push(diffs, "layers", f"count={len(jl)}", f"count={len(rl)}")
        for i in range(min(len(jl), len(rl))):
            if len(jl[i]) != len(rl[i]):
                self._push(diffs, f"layers[{i}]",
                            f"count={len(jl[i])}", f"count={len(rl[i])}")

    def _graph_size_norm(self, jn: dict, rn: dict, diffs: list[DiffEntry]) -> None:
        jg = jn.get("graphSize")
        rg = rn.get("graphSize")
        if jg is None and rg is None:
            return
        # Skip when Java has no graphSize (import-state: not set until later)
        if jg is None and rg is not None:
            return
        if rg is None:
            self._push(diffs, "graphSize", jg, rg)
            return
        for f in ("width", "height"):
            self._num(jg, rg, f, "graphSize", diffs)

    def _padding_norm(self, jn: dict, rn: dict, diffs: list[DiffEntry]) -> None:
        jp = jn.get("padding")
        rp = rn.get("padding")
        if jp is None and rp is None:
            return
        # Skip when Java has no padding (import-state: not set until later)
        if jp is None and rp is not None:
            return
        if rp is None:
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
            # Skip geometric diffs when Java reports 0 (import-state artifact:
            # Java LNodes start with all geometry=0, Rust preserves ElkNode values).
            # After LabelAndNodeSizeProcessor, Java will have real values.
            if field in ("x", "y", "width", "height") and float(jv) == 0.0 and float(rv) != 0.0:
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
    jmap: dict[int, tuple[str, Path]] = {i: (s, p) for i, s, p in java_steps if i >= 0}
    rmap: dict[int, tuple[str, Path]] = {i: (s, p) for i, s, p in rust_steps if i >= 0}

    all_indices = sorted(set(jmap) | set(rmap))
    step_results: list[StepResult] = []
    diverged = False

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
        _r_stem, r_path = rmap[idx]
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

        # Use processor name from snapshot if available
        proc = j_snap.get("processor") or proc
        diffs = comparator.compare(j_snap, r_snap)

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

def print_batch_results(results: list[ModelResult]) -> None:
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
    print(bold(f"Batch summary: {n_match} match, {n_drift} drift out of {len(results)} models"))


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
            normalized = rel.replace(".json", "")
            result[normalized] = Path(dirpath)
    return result


def iter_model_pairs(java_base: Path, rust_base: Path) -> Iterator[tuple[str, Path, Path]]:
    """Yield (model_name, java_dir, rust_dir) for each common model trace directory."""
    jmodels = _find_trace_dirs(java_base)
    rmodels = _find_trace_dirs(rust_base)
    for name in sorted(set(jmodels) & set(rmodels)):
        yield name, jmodels[name], rmodels[name]


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
      perf/model_parity/java/trace/verticalOrder/ \\
      perf/model_parity/rust/trace/verticalOrder/

  # Single model, verbose diff output
  python scripts/compare_phase_traces.py \\
      perf/model_parity/java/trace/verticalOrder/ \\
      perf/model_parity/rust/trace/verticalOrder/ \\
      --verbose

  # Stop at first divergence
  python scripts/compare_phase_traces.py \\
      perf/model_parity/java/trace/verticalOrder/ \\
      perf/model_parity/rust/trace/verticalOrder/ \\
      --stop-at-first

  # JSON output
  python scripts/compare_phase_traces.py \\
      perf/model_parity/java/trace/verticalOrder/ \\
      perf/model_parity/rust/trace/verticalOrder/ \\
      --json

  # Batch over all models
  python scripts/compare_phase_traces.py \\
      perf/model_parity/java/trace/ \\
      perf/model_parity/rust/trace/ \\
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

    comparator = SnapshotComparator(tolerance=args.tolerance, max_diffs=args.max_diffs)
    batch_mode = args.batch or _is_batch_dir(java_base, rust_base)

    # ------------------------------------------------------------------
    # Batch mode
    if batch_mode:
        results: list[ModelResult] = []
        for model_name, jdir, rdir in iter_model_pairs(java_base, rust_base):
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
                },
            }
            print(json.dumps(out, indent=2))
        else:
            print_batch_results(results)

        return 0 if all(r.first_div_step is None for r in results) else 1

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
    sys.exit(main())
