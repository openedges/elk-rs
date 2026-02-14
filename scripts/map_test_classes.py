#!/usr/bin/env python3
"""Map Java test classes to Rust test files."""

import os
import re
from pathlib import Path
from typing import Dict, List, Tuple

def extract_java_class_name(file_path: str) -> str:
    """Extract the public class/interface name from a Java file."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            for line in f:
                match = re.match(r'^public\s+(class|interface)\s+(\w+)', line)
                if match:
                    return match.group(2)
    except Exception:
        pass
    return ""

def find_java_tests(base_dir: str) -> List[Tuple[str, str, str]]:
    """Find all Java test files and extract their class names.

    Returns: List of (module, class_name, file_path)
    """
    results = []
    test_dirs = [
        "org.eclipse.elk.alg.test",
        "org.eclipse.elk.shared.test"
    ]

    for test_dir in test_dirs:
        test_path = Path(base_dir) / "external/elk/test" / test_dir
        if not test_path.exists():
            continue

        for java_file in test_path.rglob("*.java"):
            class_name = extract_java_class_name(str(java_file))
            if class_name:
                results.append((test_dir, class_name, str(java_file)))

    return sorted(results)

def snake_case(name: str) -> str:
    """Convert CamelCase to snake_case."""
    # Insert underscore before uppercase letters
    s1 = re.sub('(.)([A-Z][a-z]+)', r'\1_\2', name)
    # Insert underscore before uppercase in sequences
    return re.sub('([a-z0-9])([A-Z])', r'\1_\2', s1).lower()

def find_rust_match(class_name: str, rust_files: List[str]) -> Tuple[str, str]:
    """Find matching Rust test file.

    Returns: (status, rust_file_path)
    """
    # Manual mappings for special cases
    manual_mappings = {
        "DirectLayoutTest": "shared_direct_plain_layout_test.rs",
        "ElkLiveExamplesTest": "shared_live_examples_smoke_test.rs",
    }

    if class_name in manual_mappings:
        target = manual_mappings[class_name]
        for rust_file in rust_files:
            if rust_file.endswith(target):
                return ("mapped", rust_file)

    # Convert class name to potential Rust file names
    snake = snake_case(class_name)

    # Remove "Test" suffix if present
    if snake.endswith("_test"):
        snake_base = snake[:-5]  # Remove "_test"
    else:
        snake_base = snake

    # Search patterns (in order of preference)
    patterns = [
        f"{snake}.rs",           # exact_match_test.rs
        f"{snake_base}_test.rs", # exact_match_test.rs
        f"{snake_base}.rs",      # exact_match.rs (if no _test suffix)
    ]

    for pattern in patterns:
        for rust_file in rust_files:
            if rust_file.endswith(pattern):
                return ("mapped", rust_file)

    # Try partial match on base name
    for rust_file in rust_files:
        rust_name = os.path.basename(rust_file)
        if snake_base in rust_name or rust_name.replace("_test.rs", "") == snake_base:
            return ("mapped", rust_file)

    return ("missing", "")

def main():
    base_dir = "/Users/luuvish/Projects/research/elk-rs"

    # Find all Java test classes
    java_tests = find_java_tests(base_dir)

    # Find all Rust test files
    rust_files = []
    plugins_dir = Path(base_dir) / "plugins"
    for rust_file in plugins_dir.rglob("*.rs"):
        if "test" in str(rust_file):
            rust_files.append(str(rust_file))

    # Create output directory
    output_dir = Path(base_dir) / "perf/test_parity"
    output_dir.mkdir(parents=True, exist_ok=True)
    output_file = output_dir / "alg_shared_test_mapping.tsv"

    # Generate mapping
    with open(output_file, 'w', encoding='utf-8') as f:
        # Write header
        f.write("java_module\tjava_class\tjava_file\trust_match\trust_file\tstatus\n")

        # Write mappings
        for module, class_name, java_file in java_tests:
            status, rust_file = find_rust_match(class_name, rust_files)

            # Make paths relative
            java_rel = os.path.relpath(java_file, base_dir)
            rust_rel = os.path.relpath(rust_file, base_dir) if rust_file else ""

            f.write(f"{module}\t{class_name}\t{java_rel}\t{rust_rel if rust_rel else 'N/A'}\t{rust_file if rust_file else 'N/A'}\t{status}\n")

    print(f"Mapping created: {output_file}")

    # Print summary
    mapped = sum(1 for _, class_name, _ in java_tests if find_rust_match(class_name, rust_files)[0] == "mapped")
    total = len(java_tests)
    print(f"Total Java classes: {total}")
    print(f"Mapped to Rust: {mapped}")
    print(f"Missing in Rust: {total - mapped}")

    # Print breakdown by type
    actual_tests = [c for _, c, f in java_tests if c.endswith("Test")]
    framework = [c for _, c, f in java_tests if not c.endswith("Test")]
    mapped_tests = sum(1 for c in actual_tests if find_rust_match(c, rust_files)[0] == "mapped")
    mapped_framework = sum(1 for c in framework if find_rust_match(c, rust_files)[0] == "mapped")

    print(f"\nBreakdown:")
    print(f"  Actual test classes (*Test): {len(actual_tests)} total, {mapped_tests} mapped")
    print(f"  Framework classes: {len(framework)} total, {mapped_framework} mapped")

if __name__ == "__main__":
    main()
