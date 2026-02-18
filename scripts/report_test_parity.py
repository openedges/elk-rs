#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import subprocess
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
JAVA_METHOD_RE = re.compile(
    r"@Test\s+(?:public\s+)?void\s+([a-zA-Z0-9_]+)\s*\(",
    re.MULTILINE,
)
JAVA_GRAPH_RESOURCE_PROVIDER_RE = re.compile(r"@GraphResourceProvider\b", re.MULTILINE)
JAVA_MODEL_RESOURCE_PATH_RE = re.compile(
    r'new\s+ModelResourcePath\s*\(\s*"([^"]+)"\s*\)',
    re.MULTILINE,
)
RUST_RESOURCE_LITERAL_RE = re.compile(
    r'"([^"\n]*\.(?:elkt|elkg))"',
    re.IGNORECASE,
)


def normalize(name: str) -> str:
    # Remove common suffix/prefix 'test' and non-alphanumerics.
    lowered = name.lower()
    lowered = re.sub(r"test", "", lowered)
    return re.sub(r"[^a-z0-9]", "", lowered)


def java_tests(root: Path) -> Tuple[List[Dict[str, str]], List[Dict[str, str]]]:
    """Return (class_rows, method_rows)."""
    class_rows = []
    method_rows = []
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
        class_rows.append(
            {
                "project": project,
                "package": package,
                "class": class_name,
                "file": str(path),
                "norm": normalize(class_name),
            }
        )
        # Extract @Test method names
        for m in JAVA_METHOD_RE.finditer(stripped):
            method_name = m.group(1)
            method_rows.append(
                {
                    "project": project,
                    "package": package,
                    "class": class_name,
                    "method": method_name,
                    "file": str(path),
                    "norm": normalize(method_name),
                }
            )
    return class_rows, method_rows


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


def extract_issue_id(resource_path: str) -> str:
    name = Path(resource_path).name.lower()
    match = re.search(r"^(\d+)_", name)
    if match:
        return match.group(1)
    match = re.search(r"issue[_-]?(\d+)", name)
    if match:
        return match.group(1)
    return ""


def normalize_resource_path(resource_path: str) -> str:
    normalized = resource_path.replace("\\", "/").strip()
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized


def java_graph_resources(root: Path) -> List[Dict[str, str]]:
    rows: List[Dict[str, str]] = []
    for path in sorted(root.rglob("*.java")):
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        stripped = JAVA_BLOCK_COMMENT_RE.sub("", text)
        stripped = JAVA_LINE_COMMENT_RE.sub("", stripped)
        if not JAVA_GRAPH_RESOURCE_PROVIDER_RE.search(stripped):
            continue

        m_class = JAVA_CLASS_RE.search(stripped)
        class_name = m_class.group(1) if m_class else path.stem
        package = ""
        m_pkg = JAVA_PACKAGE_RE.search(stripped)
        if m_pkg:
            package = m_pkg.group(1)
        rel = path.relative_to(root)
        project = rel.parts[0] if rel.parts else ""

        resources = JAVA_MODEL_RESOURCE_PATH_RE.findall(stripped)
        if not resources:
            rows.append(
                {
                    "project": project,
                    "package": package,
                    "class": class_name,
                    "file": str(path),
                    "resource_pattern": "<dynamic>",
                    "issue_id": "",
                }
            )
            continue

        for resource_pattern in resources:
            rows.append(
                {
                    "project": project,
                    "package": package,
                    "class": class_name,
                    "file": str(path),
                    "resource_pattern": resource_pattern,
                    "issue_id": extract_issue_id(resource_pattern),
                }
            )
    return rows


def rust_graph_resource_literals(root: Path) -> List[Dict[str, str]]:
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

        literals = sorted(set(RUST_RESOURCE_LITERAL_RE.findall(text)))
        for literal in literals:
            normalized = normalize_resource_path(literal)
            rows.append(
                {
                    "crate": crate,
                    "file": str(path),
                    "resource_literal": literal,
                    "resource_norm": normalized,
                    "basename": Path(normalized).name,
                    "issue_id": extract_issue_id(normalized),
                }
            )
    return rows


def models_resource_stats(models_root: Path, resource_pattern: str) -> Tuple[str, int]:
    if resource_pattern == "<dynamic>":
        return "dynamic", 0
    normalized = normalize_resource_path(resource_pattern)
    if "*" in normalized:
        matches = list(models_root.glob(normalized))
        return ("yes" if matches else "no", len(matches))
    target = models_root / normalized
    return ("yes" if target.exists() else "no", 1 if target.exists() else 0)


def map_graph_resources(
    java_rows: List[Dict[str, str]],
    rust_rows: List[Dict[str, str]],
    models_root: Path,
) -> List[Dict[str, str]]:
    rust_by_basename: Dict[str, List[Dict[str, str]]] = defaultdict(list)
    rust_by_issue: Dict[str, List[Dict[str, str]]] = defaultdict(list)
    for row in rust_rows:
        rust_by_basename[row["basename"]].append(row)
        if row["issue_id"]:
            rust_by_issue[row["issue_id"]].append(row)

    mapped_rows: List[Dict[str, str]] = []
    for row in java_rows:
        pattern = row["resource_pattern"]
        normalized = normalize_resource_path(pattern)
        issue_id = row["issue_id"]
        wildcard = "*" in normalized

        exact_matches = []
        basename_matches = []
        issue_matches = []
        wildcard_matches = []
        status = "no_rust_match"

        if pattern == "<dynamic>":
            status = "dynamic_provider"
        else:
            basename = Path(normalized).name
            basename_matches = rust_by_basename.get(basename, [])
            if issue_id:
                issue_matches = rust_by_issue.get(issue_id, [])

            for rust_row in rust_rows:
                resource_norm = rust_row["resource_norm"]
                if resource_norm == normalized or resource_norm.endswith("/" + normalized):
                    exact_matches.append(rust_row)

            if wildcard:
                wildcard_prefix = normalized.split("*", 1)[0]
                for rust_row in rust_rows:
                    resource_norm = rust_row["resource_norm"]
                    if wildcard_prefix and wildcard_prefix in resource_norm:
                        wildcard_matches.append(rust_row)

            if exact_matches:
                status = "exact_path_match"
            elif wildcard and wildcard_matches:
                status = "wildcard_prefix_match"
            elif basename_matches:
                status = "basename_match"
            elif issue_matches:
                status = "issue_id_match"

        model_exists, model_match_count = models_resource_stats(models_root, pattern)
        match_pool = (
            exact_matches
            if exact_matches
            else wildcard_matches
            if wildcard_matches
            else basename_matches
            if basename_matches
            else issue_matches
        )
        rust_examples = ";".join(
            f"{match['crate']}:{Path(match['file']).name}:{match['resource_literal']}"
            for match in match_pool[:3]
        )

        mapped_rows.append(
            {
                "java_project": row["project"],
                "java_class": row["class"],
                "java_file": row["file"],
                "resource_pattern": pattern,
                "resource_issue_id": issue_id,
                "resource_wildcard": "yes" if wildcard else "no",
                "models_exists": model_exists,
                "models_match_count": str(model_match_count),
                "mapping_status": status,
                "rust_match_count": str(len(match_pool)),
                "rust_examples": rust_examples,
            }
        )

    return mapped_rows


CARGO_TEST_LINE_RE = re.compile(
    r"^test\s+(\S+)\s+\.\.\.\s+(ok|FAILED|ignored)$", re.MULTILINE
)


def capture_cargo_test_results(out_dir: Path) -> List[Dict[str, str]]:
    """Run cargo test --workspace and parse per-test results."""
    print("Running cargo test --workspace (this may take a while)...")
    result = subprocess.run(
        ["cargo", "test", "--workspace"],
        capture_output=True,
        text=True,
        timeout=600,
    )
    output = result.stdout + result.stderr
    rows = []
    for m in CARGO_TEST_LINE_RE.finditer(output):
        full_name = m.group(1)
        status = m.group(2)
        # Split full_name into crate::module::test_name
        parts = full_name.split("::")
        test_name = parts[-1] if parts else full_name
        module = "::".join(parts[:-1]) if len(parts) > 1 else ""
        rows.append({
            "full_name": full_name,
            "module": module,
            "test_name": test_name,
            "status": status,
            "norm": normalize(test_name),
        })

    with (out_dir / "rust_test_results.tsv").open("w", encoding="utf-8") as f:
        f.write("full_name\tmodule\ttest_name\tstatus\tnorm\n")
        for row in rows:
            f.write("\t".join([row["full_name"], row["module"], row["test_name"], row["status"], row["norm"]]) + "\n")

    total = len(rows)
    passed = sum(1 for r in rows if r["status"] == "ok")
    failed = sum(1 for r in rows if r["status"] == "FAILED")
    ignored = sum(1 for r in rows if r["status"] == "ignored")
    print(f"cargo test: total={total} passed={passed} failed={failed} ignored={ignored}")

    with (out_dir / "summary.txt").open("a", encoding="utf-8") as f:
        f.write(f"\n--- Cargo test results ---\n")
        f.write(f"total={total} passed={passed} failed={failed} ignored={ignored}\n")

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
    parser.add_argument("--models-root", default="external/elk-models", help="ELK models root for GraphResourceProvider path validation")
    parser.add_argument("--out-dir", default="perf/test_parity", help="Output directory")
    parser.add_argument("--capture-cargo-test", action="store_true", help="Run cargo test and capture results to rust_test_results.tsv")
    args = parser.parse_args()

    java_root = Path(args.java_root)
    rust_root = Path(args.rust_root)
    models_root = Path(args.models_root)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    java_rows, java_method_rows = java_tests(java_root)
    rust_rows = rust_tests(rust_root)

    rust_by_norm: Dict[str, List[Dict[str, str]]] = defaultdict(list)
    for row in rust_rows:
        if row["norm"]:
            rust_by_norm[row["norm"]].append(row)

    # Build rust fn-only index for method mapping
    rust_fns_by_norm: Dict[str, List[Dict[str, str]]] = defaultdict(list)
    for row in rust_rows:
        if row["kind"] == "fn" and row["norm"]:
            rust_fns_by_norm[row["norm"]].append(row)

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

    # Method-level mapping (Step T-1)
    method_matched = 0
    method_matched_same_module = 0
    method_mapping_rows = []
    for row in java_method_rows:
        norm = row["norm"]
        candidates = rust_fns_by_norm.get(norm, [])
        if candidates:
            method_matched += 1
        java_project = row["project"]
        expected_crate = project_to_crate(java_project)
        same_module = [c for c in candidates if c["crate"] == expected_crate]
        if same_module:
            method_matched_same_module += 1
        matches = ";".join(
            [f"{c['name']}@{c['crate']}" for c in candidates]
        )
        same_module_matches = ";".join(
            [f"{c['name']}@{c['crate']}" for c in same_module]
        )
        method_mapping_rows.append(
            (
                java_project,
                row["class"],
                row["method"],
                row["file"],
                norm,
                matches,
                same_module_matches,
            )
        )

    with (out_dir / "method_mapping.tsv").open("w", encoding="utf-8") as f:
        f.write("java_project\tjava_class\tjava_method\tjava_file\tnorm\trust_matches\trust_matches_same_module\n")
        for rec in method_mapping_rows:
            f.write("\t".join(rec) + "\n")

    total_java_methods = len(java_method_rows)
    method_missing = total_java_methods - method_matched

    with (out_dir / "summary.txt").open("a", encoding="utf-8") as f:
        f.write(f"\n--- Method-level parity ---\n")
        f.write(f"java_test_methods={total_java_methods}\n")
        f.write(f"method_matched_by_name={method_matched}\n")
        f.write(f"method_matched_same_module={method_matched_same_module}\n")
        f.write(f"method_missing_by_name={method_missing}\n")

    # Step T-4: GraphResourceProvider resource parity
    java_resource_rows = java_graph_resources(java_root)
    rust_resource_rows = rust_graph_resource_literals(rust_root)
    graph_resource_mapping = map_graph_resources(java_resource_rows, rust_resource_rows, models_root)

    with (out_dir / "graph_resource_mapping.tsv").open("w", encoding="utf-8") as f:
        f.write(
            "java_project\tjava_class\tjava_file\tresource_pattern\tresource_issue_id\tresource_wildcard\t"
            "models_exists\tmodels_match_count\tmapping_status\trust_match_count\trust_examples\n"
        )
        for row in graph_resource_mapping:
            f.write(
                "\t".join(
                    [
                        row["java_project"],
                        row["java_class"],
                        row["java_file"],
                        row["resource_pattern"],
                        row["resource_issue_id"],
                        row["resource_wildcard"],
                        row["models_exists"],
                        row["models_match_count"],
                        row["mapping_status"],
                        row["rust_match_count"],
                        row["rust_examples"],
                    ]
                )
                + "\n"
            )

    status_counter = Counter(row["mapping_status"] for row in graph_resource_mapping)
    mapped_count = sum(
        1
        for row in graph_resource_mapping
        if row["mapping_status"]
        in {"exact_path_match", "wildcard_prefix_match", "basename_match", "issue_id_match"}
    )
    unresolved_count = status_counter.get("no_rust_match", 0)
    dynamic_count = status_counter.get("dynamic_provider", 0)
    model_missing_count = sum(
        1 for row in graph_resource_mapping if row["models_exists"] == "no"
    )

    with (out_dir / "graph_resource_summary.md").open("w", encoding="utf-8") as f:
        f.write("# Graph Resource Parity\n\n")
        f.write(f"- java graph resources: **{len(graph_resource_mapping)}**\n")
        f.write(f"- mapped to rust resources: **{mapped_count}**\n")
        f.write(f"- unresolved: **{unresolved_count}**\n")
        f.write(f"- dynamic providers: **{dynamic_count}**\n")
        f.write(f"- missing in external/elk-models: **{model_missing_count}**\n\n")
        f.write("| Status | Count |\n")
        f.write("|---|---:|\n")
        for status, count in status_counter.most_common():
            f.write(f"| {status} | {count} |\n")

    with (out_dir / "summary.txt").open("a", encoding="utf-8") as f:
        f.write(f"\n--- GraphResourceProvider parity ---\n")
        f.write(f"java_graph_resources={len(graph_resource_mapping)}\n")
        f.write(f"graph_resources_mapped={mapped_count}\n")
        f.write(f"graph_resources_unresolved={unresolved_count}\n")
        f.write(f"graph_resources_dynamic={dynamic_count}\n")
        f.write(f"graph_resources_missing_in_models={model_missing_count}\n")
        for status, count in status_counter.most_common():
            f.write(f"graph_status_{status}={count}\n")

    # Step T-2: Capture cargo test results
    if args.capture_cargo_test:
        capture_cargo_test_results(out_dir)

    print("Test parity report written to", out_dir)
    print(f"java_tests={total_java} matched_by_name={matched} missing={missing}")
    print(f"matched_same_module={matched_same_module}")
    print(f"java_test_methods={total_java_methods} method_matched={method_matched} method_missing={method_missing}")
    print(f"method_matched_same_module={method_matched_same_module}")
    print(
        "graph_resources="
        f"{len(graph_resource_mapping)} mapped={mapped_count} "
        f"unresolved={unresolved_count} dynamic={dynamic_count}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
