#!/usr/bin/env python3
"""Generate a unified model manifest by scanning the models directory.

Scans elk-models for all model files (.elkt, .elkg, .json), discovers
Java-converted inputs and layout outputs from the Java output directory,
and produces a single manifest TSV for the Rust layout runner.

Usage:
    python3 scripts/generate_full_trace_manifest.py \
        --models-root external/elk-models \
        --java-output-dir tests/model_parity_full/java \
        --output tests/model_parity_full/java/java_manifest.tsv
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

HEADER = "model_rel_path\tinput_json\tjava_layout_json\tjava_status\tjava_error"
MODEL_EXTENSIONS = {".elkt", ".elkg", ".json"}


def scan_models(models_root: Path) -> list[tuple[str, Path]]:
    """Scan for all model files, return (rel_path, abs_path) sorted."""
    results: list[tuple[str, Path]] = []
    for dirpath, _dirnames, filenames in os.walk(models_root):
        for fname in filenames:
            ext = os.path.splitext(fname)[1].lower()
            if ext not in MODEL_EXTENSIONS:
                continue
            abs_path = (Path(dirpath) / fname).resolve()
            rel_path = os.path.relpath(abs_path, models_root)
            results.append((rel_path, abs_path))
    results.sort(key=lambda t: t[0])
    return results


def build_manifest(
    models_root: Path,
    java_output_dir: Path | None,
    exclude_paths: set[str],
) -> list[str]:
    """Build manifest rows from directory scan."""
    java_input_dir = java_output_dir / "input" if java_output_dir else None
    java_layout_dir = java_output_dir / "layout" if java_output_dir else None

    rows: list[str] = []
    for rel_path, abs_path in scan_models(models_root):
        if rel_path in exclude_paths:
            continue

        ext = abs_path.suffix.lower()

        # Java always writes to {relPath}.json for both input and layout
        json_suffix = f"{rel_path}.json"

        # Determine input JSON path
        if java_input_dir:
            candidate = java_input_dir / json_suffix
            if candidate.exists():
                input_json = str(candidate)
            elif ext == ".json":
                # .json models can be used directly as input
                input_json = str(abs_path)
            else:
                input_json = ""
        elif ext == ".json":
            input_json = str(abs_path)
        else:
            input_json = ""

        # Determine Java layout output path
        java_layout_json = ""
        if java_layout_dir:
            candidate = java_layout_dir / json_suffix
            if candidate.exists():
                java_layout_json = str(candidate)

        # Status based on what's available
        if not input_json:
            java_status = "error"
        else:
            java_status = "ok"

        rows.append(f"{rel_path}\t{input_json}\t{java_layout_json}\t{java_status}\t")

    return rows


def read_exclude_file(path: Path) -> set[str]:
    """Read exclude file (one model path per line, # comments)."""
    excludes: set[str] = set()
    if not path.exists():
        return excludes
    with open(path, "r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line and not line.startswith("#"):
                excludes.add(line)
    return excludes


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate unified model manifest by scanning models directory."
    )
    parser.add_argument(
        "--models-root", required=True, type=Path,
        help="Root directory of elk-models (e.g., external/elk-models)"
    )
    parser.add_argument(
        "--java-output-dir", type=Path, default=None,
        help="Java output directory containing input/ and layout/ subdirs"
    )
    parser.add_argument(
        "--output", required=True, type=Path,
        help="Output path for the manifest TSV"
    )
    parser.add_argument(
        "--exclude-file", type=Path, default=None,
        help="File listing model paths to exclude (one per line)"
    )
    args = parser.parse_args()

    if not args.models_root.is_dir():
        print(f"error: models root not found: {args.models_root}", file=sys.stderr)
        return 2

    excludes = read_exclude_file(args.exclude_file) if args.exclude_file else set()
    rows = build_manifest(args.models_root, args.java_output_dir, excludes)

    if args.output.parent and not args.output.parent.exists():
        args.output.parent.mkdir(parents=True, exist_ok=True)

    with open(args.output, "w", encoding="utf-8") as fh:
        fh.write(HEADER + "\n")
        for row in rows:
            fh.write(row + "\n")

    has_java = sum(1 for r in rows if r.split("\t")[2])  # java_layout_json non-empty
    has_input = sum(1 for r in rows if r.split("\t")[1])  # input_json non-empty
    no_input = len(rows) - has_input
    print(f"Manifest: {len(rows)} models (input={has_input}, java_layout={has_java}, no_input={no_input})")
    print(f"Written to: {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
