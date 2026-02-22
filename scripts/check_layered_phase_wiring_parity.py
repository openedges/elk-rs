#!/usr/bin/env python3
"""Compare layered phase wiring parity between Java and Rust GraphConfigurator."""

from __future__ import annotations

import argparse
import csv
import re
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass(frozen=True)
class WiringRow:
    placement: str
    phase: str
    processor: str
    guard: str


@dataclass(frozen=True)
class Action:
    kind: str  # row | add_all
    placement: str
    phase: str
    processor: str
    config: str
    guard: str
    order: int


def camel_to_upper_snake(value: str) -> str:
    value = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", value)
    value = re.sub(r"([A-Za-z])([0-9])", r"\1_\2", value)
    return value.upper()


def normalize_phase(lang: str, phase: str) -> str:
    if lang == "java":
        return phase.strip().upper()
    normalized = camel_to_upper_snake(phase.strip())
    return re.sub(r"^P_([0-9])_", r"P\1_", normalized)


def normalize_processor(token: str) -> str:
    token = token.strip()
    if token in {"internalGreedyType", "internal_strategy"}:
        return "DYNAMIC_GREEDY_SWITCH"

    if "IntermediateProcessorStrategy." in token:
        token = token.split(".")[-1]
    elif "IntermediateProcessorStrategy::" in token:
        token = token.split("::")[-1]
    else:
        token = token.split(".")[-1]
        token = token.split("::")[-1]

    return camel_to_upper_snake(token)


def dedupe_keep_order(values: list[str]) -> list[str]:
    result: list[str] = []
    seen: set[str] = set()
    for value in values:
        if not value or value in seen:
            continue
        seen.add(value)
        result.append(value)
    return result


def guard_signature(tags: list[str]) -> str:
    normalized = sorted(set(tag for tag in tags if tag and tag != "always"))
    if not normalized:
        return "always"
    return " && ".join(normalized)


def merge_guards(base: str, extra: str) -> str:
    tags: list[str] = []
    if base != "always":
        tags.extend(base.split(" && "))
    if extra != "always":
        tags.extend(extra.split(" && "))
    return guard_signature(tags)


def invert_guard_tag(tag: Optional[str]) -> Optional[str]:
    if tag == "feedback_edges_true":
        return "feedback_edges_false"
    return None


def simplified_key(text: str) -> str:
    return re.sub(r"[^a-z0-9_]+", "", text.lower().replace("::", "_").replace(".", "_"))


def map_if_guard(header_text: str) -> Optional[str]:
    key = simplified_key(header_text)
    compact = key.replace("_", "")

    if "feedbackedges" in compact:
        return "feedback_edges_true"
    if "labelmanager" in compact:
        return "label_manager_present"
    if "interactivelayout" in compact or "generatepositionandlayerids" in compact:
        return "interactive_or_generate_ids"
    if "hierarchyhandling" in compact and "includechildren" in compact:
        return "hierarchy_include_children"
    if "graphproperties" in compact and "comments" in compact:
        return "graph_properties_comments"
    if "layeringnodepromotionstrategy" in compact:
        return "node_promotion_non_none"
    if "graphproperties" in compact and "partitions" in compact:
        return "graph_properties_partitions"
    if "compactionpostcompactionstrategy" in compact:
        return "compaction_non_none_and_not_polyline"
    if "highdegreenodestreatment" in compact:
        return "high_degree_nodes_treatment"
    if "crossingminimizationsemiinteractive" in compact and "interactivecrossmin" not in compact:
        return "crossmin_semi_interactive"
    if "activategreedyswitchfor" in compact:
        return "greedy_switch_active"
    if "layerunzippingstrategy" in compact and "alternating" in compact:
        return "layer_unzipping_alternating"
    if "considermodelorderstrategy" in compact:
        return "consider_model_order_non_none"

    return None


def map_java_switch_key(header_text: str) -> Optional[str]:
    key = simplified_key(header_text)
    if "layeredoptions_direction" in key:
        return "direction"
    if "layeredoptions_wrapping_strategy" in key:
        return "wrapping"
    if "layeredoptions_layer_unzipping_strategy" in key:
        return "layer_unzipping"
    return None


def map_java_case_tag(switch_key: str, case_name: str) -> Optional[str]:
    case_name = case_name.strip().upper()
    if switch_key == "direction" and case_name in {"LEFT", "DOWN", "UP"}:
        return "direction_left_down_up"
    if switch_key == "wrapping" and case_name == "SINGLE_EDGE":
        return "wrapping_single_edge"
    if switch_key == "wrapping" and case_name == "MULTI_EDGE":
        return "wrapping_multi_edge"
    if switch_key == "layer_unzipping" and case_name == "ALTERNATING":
        return "layer_unzipping_alternating"
    return None


def map_rust_match_key(header_text: str) -> Optional[str]:
    key = simplified_key(header_text)
    if "layeredoptions_direction" in key:
        return "direction"
    if "layeredoptions_wrapping_strategy" in key:
        return "wrapping"
    return None


def map_rust_arm_tag(match_key: str, line: str) -> Optional[str]:
    if match_key == "direction":
        if "Direction::Left" in line or "Direction::Down" in line or "Direction::Up" in line:
            return "direction_left_down_up"
        return None
    if match_key == "wrapping":
        if "WrappingStrategy::SingleEdge" in line:
            return "wrapping_single_edge"
        if "WrappingStrategy::MultiEdge" in line:
            return "wrapping_multi_edge"
        return None
    return None


def extract_brace_block(text: str, signature_pattern: str) -> tuple[str, int]:
    match = re.search(signature_pattern, text)
    if match is None:
        raise RuntimeError(f"cannot find function signature: {signature_pattern}")

    brace_start = text.find("{", match.end())
    if brace_start < 0:
        raise RuntimeError(f"cannot find opening brace after: {signature_pattern}")

    depth = 0
    for idx in range(brace_start, len(text)):
        char = text[idx]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                body = text[brace_start + 1 : idx]
                start_line = text.count("\n", 0, brace_start + 1) + 1
                return body, start_line

    raise RuntimeError(f"unterminated block for: {signature_pattern}")


def extract_java_static_block(text: str, name: str) -> str:
    pattern = (
        rf"private\s+static\s+final[^\n]*\b{name}\b\s*="
        rf"\s*LayoutProcessorConfiguration.*?;"
    )
    match = re.search(pattern, text, re.DOTALL)
    if match is None:
        raise RuntimeError(f"cannot find Java static config: {name}")
    return match.group(0)


def extract_rust_static_block(text: str, name: str) -> str:
    pattern = rf"static\s+{name}\s*:[^=]*=\s*LazyLock::new\(\|\|\s*\{{.*?\}}\s*\);"
    match = re.search(pattern, text, re.DOTALL)
    if match is None:
        raise RuntimeError(f"cannot find Rust static config: {name}")
    return match.group(0)


def parse_static_rows(lang: str, block_text: str) -> list[WiringRow]:
    if lang == "java":
        pattern = re.compile(
            r"\.add(Before|After)\(\s*LayeredPhases\.([A-Z0-9_]+)\s*,\s*([A-Za-z0-9_\.]+)\s*\)",
            re.DOTALL,
        )
    else:
        pattern = re.compile(
            r"\.add_(before|after)\(\s*LayeredPhases::([A-Za-z0-9_]+)\s*,\s*Arc::new\(\s*([A-Za-z0-9_:]+)\s*\)\s*,?\s*\)",
            re.DOTALL,
        )

    rows: list[WiringRow] = []
    for match in pattern.finditer(block_text):
        placement = match.group(1).lower()
        if placement == "before":
            normalized_placement = "before"
        else:
            normalized_placement = "after"
        phase = normalize_phase(lang, match.group(2))
        processor = normalize_processor(match.group(3))
        rows.append(
            WiringRow(
                placement=normalized_placement,
                phase=phase,
                processor=processor,
                guard="always",
            )
        )
    return rows


def finalize_pending_header(lang: str, pending: dict) -> Optional[dict]:
    kind = pending.get("kind")
    text = pending.get("text", "")

    if kind == "else":
        tag = pending.get("tag")
        if tag:
            return {"tag": tag}
        return None

    if kind == "if":
        tag = map_if_guard(text)
        if tag:
            return {"tag": tag}
        return None

    if kind == "switch":
        key = map_java_switch_key(text)
        if key:
            return {"switch_key": key, "case_tag": None}
        return None

    if kind == "match":
        key = map_rust_match_key(text)
        if key:
            return {"match_key": key}
        return None

    return None


def build_line_guards(lang: str, body_text: str, start_line: int) -> dict[int, list[str]]:
    frames: list[dict] = []
    pending_push: list[dict] = []
    pending_header: Optional[dict] = None
    last_closed_tag: Optional[str] = None
    brace_depth = 0
    line_guards: dict[int, list[str]] = {}

    lines = body_text.splitlines()
    for offset, raw_line in enumerate(lines):
        line_no = start_line + offset
        line_without_comments = re.sub(r"//.*", "", raw_line)
        stripped = line_without_comments.strip()

        tags: list[str] = []
        for frame in frames:
            tag = frame.get("tag")
            if tag:
                tags.append(tag)
            case_tag = frame.get("case_tag")
            if case_tag:
                tags.append(case_tag)
        line_guards[line_no] = dedupe_keep_order(tags)

        if lang == "java":
            switch_frame = next((f for f in reversed(frames) if f.get("switch_key")), None)
            if switch_frame is not None:
                case_match = re.match(r"case\s+([A-Z_]+)\s*:", stripped)
                if case_match:
                    switch_frame["case_tag"] = map_java_case_tag(
                        switch_frame["switch_key"], case_match.group(1)
                    )
                elif re.match(r"default\s*:", stripped):
                    switch_frame["case_tag"] = None
                elif "break;" in stripped:
                    switch_frame["case_tag"] = None

        if lang == "rust":
            match_frame = next((f for f in reversed(frames) if f.get("match_key")), None)
            if match_frame is not None and "=>" in stripped:
                arm_tag = map_rust_arm_tag(match_frame["match_key"], stripped)
                if arm_tag:
                    pending_push.append({"tag": arm_tag})

        if pending_header is not None:
            if stripped:
                pending_header["text"] = (pending_header.get("text", "") + " " + stripped).strip()
            if "{" in stripped:
                frame = finalize_pending_header(lang, pending_header)
                if frame is not None:
                    pending_push.append(frame)
                pending_header = None
        else:
            else_match = re.search(r"\belse\b", stripped)
            else_if_match = re.search(r"\belse\s+if\b", stripped)
            if else_match and not else_if_match:
                source_tag = last_closed_tag
                if "} else" in stripped and frames:
                    source_tag = frames[-1].get("tag") or source_tag
                else_tag = invert_guard_tag(source_tag)
                if else_tag:
                    if "{" in stripped:
                        pending_push.append({"tag": else_tag})
                    else:
                        pending_header = {"kind": "else", "tag": else_tag, "text": stripped}

            if pending_header is None:
                if lang == "java":
                    switch_match = re.search(r"\bswitch\s*\(", stripped)
                    if switch_match:
                        header = stripped[switch_match.start() :]
                        if "{" in stripped:
                            frame = finalize_pending_header(
                                lang, {"kind": "switch", "text": header}
                            )
                            if frame is not None:
                                pending_push.append(frame)
                        else:
                            pending_header = {"kind": "switch", "text": header}

                    if pending_header is None:
                        if_match = re.search(r"\bif\s*\(", stripped)
                        if if_match:
                            header = stripped[if_match.start() :]
                            if "{" in stripped:
                                frame = finalize_pending_header(
                                    lang, {"kind": "if", "text": header}
                                )
                                if frame is not None:
                                    pending_push.append(frame)
                            else:
                                pending_header = {"kind": "if", "text": header}

                else:
                    if re.match(r"match\b", stripped):
                        header = stripped
                        if "{" in stripped:
                            frame = finalize_pending_header(
                                lang, {"kind": "match", "text": header}
                            )
                            if frame is not None:
                                pending_push.append(frame)
                        else:
                            pending_header = {"kind": "match", "text": header}

                    if pending_header is None:
                        if re.match(r"(?:\}\s*)?if\b", stripped) or re.match(
                            r"else\s+if\b", stripped
                        ):
                            header = stripped
                            if "{" in stripped:
                                frame = finalize_pending_header(
                                    lang, {"kind": "if", "text": header}
                                )
                                if frame is not None:
                                    pending_push.append(frame)
                            else:
                                pending_header = {"kind": "if", "text": header}

        for char in line_without_comments:
            if char == "}":
                while frames and frames[-1]["depth"] == brace_depth:
                    popped = frames.pop()
                    popped_tag = popped.get("tag")
                    if popped_tag:
                        last_closed_tag = popped_tag
                brace_depth = max(brace_depth - 1, 0)
            elif char == "{":
                brace_depth += 1
                if pending_push:
                    frame = pending_push.pop(0)
                    frame["depth"] = brace_depth
                    frame.setdefault("tag", None)
                    frame.setdefault("switch_key", None)
                    frame.setdefault("case_tag", None)
                    frame.setdefault("match_key", None)
                    frames.append(frame)

    return line_guards


def parse_method_actions(
    lang: str, body_text: str, start_line: int, line_guards: dict[int, list[str]]
) -> list[Action]:
    events: list[tuple[int, Action]] = []

    if lang == "java":
        create_pattern = re.compile(r"createFrom\(\s*([A-Z_]+)\s*\)", re.DOTALL)
        add_all_pattern = re.compile(r"\.addAll\(\s*([A-Z_]+)\s*\)", re.DOTALL)
        add_pattern = re.compile(
            r"\.add(Before|After)\(\s*LayeredPhases\.([A-Z0-9_]+)\s*,\s*([A-Za-z0-9_\.]+)\s*\)",
            re.DOTALL,
        )
    else:
        create_pattern = re.compile(r"create_from\(\s*&?([A-Z_]+)\s*\)", re.DOTALL)
        add_all_pattern = re.compile(r"\.add_all\(\s*&?([A-Z_]+)\s*\)", re.DOTALL)
        add_pattern = re.compile(
            r"\.add_(before|after)\(\s*LayeredPhases::([A-Za-z0-9_]+)\s*,\s*Arc::new\(\s*([A-Za-z0-9_:]+)\s*\)\s*,?\s*\)",
            re.DOTALL,
        )

    for match in create_pattern.finditer(body_text):
        line_no = start_line + body_text.count("\n", 0, match.start())
        guard = guard_signature(line_guards.get(line_no, []))
        events.append(
            (
                match.start(),
                Action(
                    kind="add_all",
                    placement="",
                    phase="",
                    processor="",
                    config=match.group(1),
                    guard=guard,
                    order=match.start(),
                ),
            )
        )

    for match in add_all_pattern.finditer(body_text):
        line_no = start_line + body_text.count("\n", 0, match.start())
        guard = guard_signature(line_guards.get(line_no, []))
        events.append(
            (
                match.start(),
                Action(
                    kind="add_all",
                    placement="",
                    phase="",
                    processor="",
                    config=match.group(1),
                    guard=guard,
                    order=match.start(),
                ),
            )
        )

    for match in add_pattern.finditer(body_text):
        line_no = start_line + body_text.count("\n", 0, match.start())
        guard = guard_signature(line_guards.get(line_no, []))
        placement = match.group(1).lower()
        phase = normalize_phase(lang, match.group(2))
        processor = normalize_processor(match.group(3))
        events.append(
            (
                match.start(),
                Action(
                    kind="row",
                    placement="before" if placement == "before" else "after",
                    phase=phase,
                    processor=processor,
                    config="",
                    guard=guard,
                    order=match.start(),
                ),
            )
        )

    events.sort(key=lambda item: item[0])
    return [action for _, action in events]


def expand_actions(
    actions: list[Action], static_configs: dict[str, list[WiringRow]]
) -> tuple[list[WiringRow], Counter[str]]:
    rows: list[WiringRow] = []
    missing_configs: Counter[str] = Counter()

    for action in actions:
        if action.kind == "row":
            rows.append(
                WiringRow(
                    placement=action.placement,
                    phase=action.phase,
                    processor=action.processor,
                    guard=action.guard,
                )
            )
            continue

        config_rows = static_configs.get(action.config)
        if not config_rows:
            missing_configs[action.config] += 1
            continue

        for base in config_rows:
            rows.append(
                WiringRow(
                    placement=base.placement,
                    phase=base.phase,
                    processor=base.processor,
                    guard=merge_guards(action.guard, base.guard),
                )
            )

    return rows, missing_configs


def row_sort_key(row: WiringRow) -> tuple[str, str, str, str]:
    return (row.placement, row.phase, row.processor, row.guard)


def write_counter_tsv(path: Path, counts: Counter[WiringRow]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t")
        writer.writerow(["placement", "phase", "processor", "guard", "count"])
        for row in sorted(counts, key=row_sort_key):
            writer.writerow([row.placement, row.phase, row.processor, row.guard, counts[row]])


def write_diff_tsv(path: Path, diffs: list[tuple[WiringRow, int, int, int]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t")
        writer.writerow(
            [
                "placement",
                "phase",
                "processor",
                "guard",
                "java_count",
                "rust_count",
                "delta",
            ]
        )
        for row, java_count, rust_count, delta in diffs:
            writer.writerow(
                [
                    row.placement,
                    row.phase,
                    row.processor,
                    row.guard,
                    java_count,
                    rust_count,
                    delta,
                ]
            )


def write_missing_configs_tsv(path: Path, missing: Counter[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t")
        writer.writerow(["config_name", "count"])
        for config_name in sorted(missing):
            writer.writerow([config_name, missing[config_name]])


def build_markdown_table(diffs: list[tuple[WiringRow, int, int, int]]) -> list[str]:
    lines: list[str] = []
    if not diffs:
        lines.append("- none")
        return lines

    lines.append("| placement | phase | processor | guard | java | rust | delta |")
    lines.append("| --- | --- | --- | --- | ---: | ---: | ---: |")
    for row, java_count, rust_count, delta in diffs:
        lines.append(
            f"| {row.placement} | {row.phase} | {row.processor} | {row.guard} | {java_count} | {rust_count} | {delta} |"
        )
    return lines


def write_report(
    report_file: Path,
    artifact_dir: Path,
    status: str,
    java_counts: Counter[WiringRow],
    rust_counts: Counter[WiringRow],
    missing_in_rust: list[tuple[WiringRow, int, int, int]],
    extra_in_rust: list[tuple[WiringRow, int, int, int]],
    java_missing_configs: Counter[str],
    rust_missing_configs: Counter[str],
) -> None:
    report_file.parent.mkdir(parents=True, exist_ok=True)

    java_total = sum(java_counts.values())
    rust_total = sum(rust_counts.values())

    lines: list[str] = [
        "# Layered Phase Wiring Parity",
        "",
        f"- status: {status}",
        f"- java expanded wiring rows: {java_total}",
        f"- rust expanded wiring rows: {rust_total}",
        f"- java distinct wiring rows: {len(java_counts)}",
        f"- rust distinct wiring rows: {len(rust_counts)}",
        f"- missing rows in rust: {len(missing_in_rust)}",
        f"- extra rows in rust: {len(extra_in_rust)}",
        f"- java unresolved add_all configs: {sum(java_missing_configs.values())}",
        f"- rust unresolved add_all configs: {sum(rust_missing_configs.values())}",
        f"- artifacts: `{artifact_dir}`",
        "",
        "## Missing Rows In Rust",
    ]
    lines.extend(build_markdown_table(missing_in_rust))
    lines.append("")
    lines.append("## Extra Rows In Rust")
    lines.extend(build_markdown_table(extra_in_rust))

    if java_missing_configs:
        lines.append("")
        lines.append("## Java Unresolved add_all Configs")
        for config_name in sorted(java_missing_configs):
            lines.append(f"- {config_name}: {java_missing_configs[config_name]}")

    if rust_missing_configs:
        lines.append("")
        lines.append("## Rust Unresolved add_all Configs")
        for config_name in sorted(rust_missing_configs):
            lines.append(f"- {config_name}: {rust_missing_configs[config_name]}")

    report_file.write_text("\n".join(lines) + "\n", encoding="utf-8")


def collect_lang_rows(lang: str, file_text: str) -> tuple[list[WiringRow], Counter[str]]:
    if lang == "java":
        static_names = [
            "BASELINE_PROCESSING_CONFIGURATION",
            "LABEL_MANAGEMENT_ADDITIONS",
            "HIERARCHICAL_ADDITIONS",
        ]
        static_blocks = {
            name: extract_java_static_block(file_text, name) for name in static_names
        }
        static_rows = {name: parse_static_rows(lang, block) for name, block in static_blocks.items()}
        body, body_start = extract_brace_block(
            file_text,
            r"private\s+LayoutProcessorConfiguration<LayeredPhases,\s*LGraph>\s+getPhaseIndependentLayoutProcessorConfiguration\s*\(",
        )
    else:
        static_names = [
            "BASELINE_PROCESSING_CONFIGURATION",
            "LABEL_MANAGEMENT_ADDITIONS",
            "HIERARCHICAL_ADDITIONS",
        ]
        static_blocks = {
            name: extract_rust_static_block(file_text, name) for name in static_names
        }
        static_rows = {name: parse_static_rows(lang, block) for name, block in static_blocks.items()}
        body, body_start = extract_brace_block(
            file_text,
            r"fn\s+get_phase_independent_layout_processor_configuration\s*\(",
        )

    line_guards = build_line_guards(lang, body, body_start)
    actions = parse_method_actions(lang, body, body_start, line_guards)
    return expand_actions(actions, static_rows)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-file", required=True)
    parser.add_argument("--rust-file", required=True)
    parser.add_argument("--report-file", required=True)
    parser.add_argument("--artifact-dir", required=True)
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    java_file = Path(args.java_file)
    rust_file = Path(args.rust_file)
    report_file = Path(args.report_file)
    artifact_dir = Path(args.artifact_dir)
    artifact_dir.mkdir(parents=True, exist_ok=True)

    java_text = java_file.read_text(encoding="utf-8")
    rust_text = rust_file.read_text(encoding="utf-8")

    java_rows, java_missing_configs = collect_lang_rows("java", java_text)
    rust_rows, rust_missing_configs = collect_lang_rows("rust", rust_text)

    java_counts = Counter(java_rows)
    rust_counts = Counter(rust_rows)

    missing_in_rust: list[tuple[WiringRow, int, int, int]] = []
    extra_in_rust: list[tuple[WiringRow, int, int, int]] = []
    for row in sorted(set(java_counts) | set(rust_counts), key=row_sort_key):
        java_count = java_counts.get(row, 0)
        rust_count = rust_counts.get(row, 0)
        if java_count > rust_count:
            missing_in_rust.append((row, java_count, rust_count, java_count - rust_count))
        elif rust_count > java_count:
            extra_in_rust.append((row, java_count, rust_count, rust_count - java_count))

    write_counter_tsv(artifact_dir / "java_rows.tsv", java_counts)
    write_counter_tsv(artifact_dir / "rust_rows.tsv", rust_counts)
    write_diff_tsv(artifact_dir / "missing_in_rust.tsv", missing_in_rust)
    write_diff_tsv(artifact_dir / "extra_in_rust.tsv", extra_in_rust)
    write_missing_configs_tsv(artifact_dir / "java_missing_configs.tsv", java_missing_configs)
    write_missing_configs_tsv(artifact_dir / "rust_missing_configs.tsv", rust_missing_configs)

    status = "ok"
    if (
        missing_in_rust
        or extra_in_rust
        or java_missing_configs
        or rust_missing_configs
    ):
        status = "drift"

    write_report(
        report_file=report_file,
        artifact_dir=artifact_dir,
        status=status,
        java_counts=java_counts,
        rust_counts=rust_counts,
        missing_in_rust=missing_in_rust,
        extra_in_rust=extra_in_rust,
        java_missing_configs=java_missing_configs,
        rust_missing_configs=rust_missing_configs,
    )

    print(f"wrote layered phase wiring parity report: {report_file}")
    print(f"wrote layered phase wiring artifacts: {artifact_dir}")

    if args.strict and status != "ok":
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
