#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
from collections import defaultdict, Counter
from pathlib import Path
from typing import Dict, List, Tuple

JAVA_CLASS_RE = re.compile(
    r"^\s*(?:public\s+)?(?:abstract\s+)?(?:final\s+)?class\s+([A-Za-z0-9_]+)",
    re.MULTILINE,
)
JAVA_PACKAGE_RE = re.compile(r"^\s*package\s+([A-Za-z0-9_.]+)\s*;", re.MULTILINE)
JAVA_BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/", re.DOTALL)
JAVA_LINE_COMMENT_RE = re.compile(r"//.*?$", re.MULTILINE)
RUST_TEST_RE = re.compile(r"^\s*#\s*\[test\]", re.MULTILINE)
RUST_FN_RE = re.compile(r"^\s*fn\s+([a-zA-Z0-9_]+)\s*\(", re.MULTILINE)
JAVA_TEST_ANNOTATION_RE = re.compile(
    r"@Test(?:AfterProcessor|BeforeProcessor)?\b",
    re.MULTILINE,
)


def normalize(name: str) -> str:
    # Remove common suffix/prefix 'test' and non-alphanumerics.
    lowered = name.lower()
    lowered = re.sub(r"test", "", lowered)
    return re.sub(r"[^a-z0-9]", "", lowered)


def java_tests(root: Path) -> List[Dict[str, str]]:
    rows = []
    for path in sorted(root.rglob("*.java")):
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        stripped = JAVA_BLOCK_COMMENT_RE.sub("", text)
        stripped = JAVA_LINE_COMMENT_RE.sub("", stripped)
        if not JAVA_TEST_ANNOTATION_RE.search(stripped):
            continue
        m_class = JAVA_CLASS_RE.search(stripped)
        if not m_class:
            continue
        class_name = m_class.group(1)
        package = ""
        m_pkg = JAVA_PACKAGE_RE.search(stripped)
        if m_pkg:
            package = m_pkg.group(1)
        rel = path.relative_to(root)
        project = rel.parts[0] if rel.parts else ""
        rows.append(
            {
                "project": project,
                "package": package,
                "class": class_name,
                "file": str(path),
                "norm": normalize(class_name),
            }
        )
    return rows


def rust_tests(root: Path) -> List[Dict[str, str]]:
    rows: List[Dict[str, str]] = []
    for path in sorted(root.rglob("tests/*.rs")):
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        crate = ""
        parts = path.parts
        if "plugins" in parts:
            idx = parts.index("plugins")
            if idx + 1 < len(parts):
                crate = parts[idx + 1]
        file_stem = path.stem
        rows.append(
            {
                "crate": crate,
                "file": str(path),
                "name": file_stem,
                "kind": "file",
                "norm": normalize(file_stem),
            }
        )
        # Collect #[test] function names
        # naive scan: find lines with #[test], then next fn
        if "#[test]" in text:
            for match in RUST_TEST_RE.finditer(text):
                # search forward for next fn
                fn_match = RUST_FN_RE.search(text, match.end())
                if fn_match:
                    fn_name = fn_match.group(1)
                    rows.append(
                        {
                            "crate": crate,
                            "file": str(path),
                            "name": fn_name,
                            "kind": "fn",
                            "norm": normalize(fn_name),
                        }
                    )
    return rows


def project_to_crate(project: str) -> str:
    if project.endswith(".test"):
        return project[: -len(".test")]
    if project.endswith(".tests"):
        return project[: -len(".tests")]
    return project


def main() -> int:
    parser = argparse.ArgumentParser(description="Map Java tests to Rust tests by normalized name.")
    parser.add_argument("--java-root", default="external/elk/test", help="Java tests root")
    parser.add_argument("--rust-root", default="plugins", help="Rust crates root")
    parser.add_argument("--out-dir", default="perf/test_parity", help="Output directory")
    args = parser.parse_args()

    java_root = Path(args.java_root)
    rust_root = Path(args.rust_root)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    java_rows = java_tests(java_root)
    rust_rows = rust_tests(rust_root)

    rust_by_norm: Dict[str, List[Dict[str, str]]] = defaultdict(list)
    for row in rust_rows:
        if row["norm"]:
            rust_by_norm[row["norm"]].append(row)

    # Write java_tests.tsv
    with (out_dir / "java_tests.tsv").open("w", encoding="utf-8") as f:
        f.write("project\tpackage\tclass\tfile\tnorm\n")
        for row in java_rows:
            f.write(
                "\t".join(
                    [
                        row["project"],
                        row["package"],
                        row["class"],
                        row["file"],
                        row["norm"],
                    ]
                )
                + "\n"
            )

    # Write rust_tests.tsv
    with (out_dir / "rust_tests.tsv").open("w", encoding="utf-8") as f:
        f.write("crate\tfile\tname\tkind\tnorm\n")
        for row in rust_rows:
            f.write(
                "\t".join(
                    [
                        row["crate"],
                        row["file"],
                        row["name"],
                        row["kind"],
                        row["norm"],
                    ]
                )
                + "\n"
            )

    # Mapping
    matched = 0
    matched_same_module = 0
    mapping_rows: List[Tuple[str, str, str, str, str, str]] = []
    for row in java_rows:
        norm = row["norm"]
        candidates = rust_by_norm.get(norm, [])
        if candidates:
            matched += 1
        java_project = row["project"]
        expected_crate = project_to_crate(java_project)
        same_module = [c for c in candidates if c["crate"] == expected_crate]
        if same_module:
            matched_same_module += 1
        matches = ";".join(
            [f"{c['name']}@{c['crate']}#{c['kind']}" for c in candidates]
        )
        same_module_matches = ";".join(
            [f"{c['name']}@{c['crate']}#{c['kind']}" for c in same_module]
        )
        mapping_rows.append(
            (
                java_project,
                row["class"],
                row["file"],
                norm,
                matches,
                same_module_matches,
            )
        )

    with (out_dir / "mapping.tsv").open("w", encoding="utf-8") as f:
        f.write("java_project\tjava_class\tjava_file\tnorm\trust_matches\trust_matches_same_module\n")
        for rec in mapping_rows:
            f.write("\t".join(rec) + "\n")

    # Summary
    total_java = len(java_rows)
    total_rust = len({(r["crate"], r["name"], r["kind"]) for r in rust_rows})
    missing = total_java - matched
    by_project = Counter(r["project"] for r in java_rows)
    missing_by_project = Counter()
    for row in java_rows:
        if not rust_by_norm.get(row["norm"]):
            missing_by_project[row["project"]] += 1

    with (out_dir / "summary.txt").open("w", encoding="utf-8") as f:
        f.write(f"java_tests={total_java}\n")
        f.write(f"rust_test_items={total_rust}\n")
        f.write(f"matched_by_name={matched}\n")
        f.write(f"matched_same_module={matched_same_module}\n")
        f.write(f"missing_by_name={missing}\n")
        f.write("\nmissing_by_project\n")
        for project, count in missing_by_project.most_common():
            f.write(f"{project}\t{count}\t/ {by_project[project]}\n")

    print("Test parity report written to", out_dir)
    print(f"java_tests={total_java} matched_by_name={matched} missing={missing}")
    print(f"matched_same_module={matched_same_module}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
