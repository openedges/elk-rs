# elk-rs Custom Features — edgeless-port-layout-fix

## Overview

This document describes the **edgeless-port-layout-fix** custom feature: switching the LGraph-level node sizing from the simplified `process_node()` path to Java's full 7-phase CellSystem `process()` pipeline. This change resolves 70 diffs in the QA customer model (OAD) while maintaining 1988/1989 model parity with Java ELK.

The switch uncovered and fixed three latent bugs in the previously untested `process()` code path.

## Branch and Version

| Item | Value |
|------|-------|
| Feature branch | `custom/edgeless-port-layout-fix` |
| Base | `main` (`v0.11.0` — ELK Java 1:1 parity) |
| Java ELK status | Now matches Java's single-pass architecture |
| QA reference | `elk-rs-qa/oad_without_edges/` |

---

## Feature: Full CellSystem Process as Default

### Description

Java's `LabelAndNodeSizeProcessor` calls `NodeDimensionCalculation.calculateLabelAndNodeSizes()` which internally uses the full 7-phase `process()` CellSystem pipeline for each node. The previous Rust implementation used a simplified `process_node()` + Step 2 workaround instead.

This change makes Rust match Java's architecture:

**Before:**
```
LabelAndNodeSizeProcessor
  → Step 1: process_node()        — simplified sizing (no CellSystem)
  → Step 2: place_ports_on_node() — Rust-only port placement workaround
  → Step 2a~2e: iterative fixup passes
```

**After:**
```
LabelAndNodeSizeProcessor
  → Step 1: process()             — Java's 7-phase CellSystem pipeline
  → (Step 2 disabled)
```

### Impact

| Metric | Before | After |
|--------|--------|-------|
| Model parity | 1988/1989 | 1988/1989 (unchanged) |
| Phase-step trace | 1996/1997, drift=1 | 1997/1997, drift=0 (improved) |
| QA OAD diffs (vs ELKJS) | 70 | 0 |
| Workspace tests | All pass | All pass |

### Opt-out

Set `ELK_NODE_DIM_SKIP_FULL_PROCESS=1` to revert to the previous `process_node()` path.

---

## Bug Fixes

Three latent bugs were discovered in the `process()` code path, which had never been the default and therefore had zero test coverage.

### Bug 1: Multi-Label Cell Overwrite (NodeLabelCellCreator)

**File**: `plugins/.../nodespacing/internal/algorithm/node_label_cell_creator.rs`

**Root cause**: `retrieve_node_label_cell()` checked `node_label_cells` HashMap for existing cells, but this HashMap was never populated. Every call created a new cell at the same grid position, overwriting the previous one.

**Symptom**: When a node had two labels at the same CellSystem location (e.g., both `INSIDE V_TOP H_CENTER`), the first label was lost — its position remained at (0, 0).

**Example** (`nodeLabelPlacement.elkt`, node N1):
```
Before fix:  label[0] "Main Node Label"   → (0, 0)     ← overwritten
             label[1] "Second Node Label"  → (41.5, 5)  ← only this survives
After fix:   label[0] "Main Node Label"   → (50, 5)    ← correctly placed
             label[1] "Second Node Label"  → (41.5, 20) ← stacked below
```

**Fix**: Check the container directly for `CellChild::Label` existence instead of relying on the unpopulated HashMap.

### Bug 2: Fixed Port Label InsidePart Zero Position (NodeLabelAndSizeUtilities)

**File**: `plugins/.../nodespacing/internal/algorithm/node_label_and_size_utilities.rs`

**Root cause**: `setup_node_padding_for_ports_with_offset()` computed the inside extension of fixed port labels using `KVector::new()` (zero) instead of the actual label positions stored in `port_context.label_positions`.

**Symptom**: For nodes with NORTH ports whose fixed labels extend below into the node, the node height was underestimated because the inside extension was computed from position (0, 0) instead of the real label position (e.g., (-40, 21)).

**Example** (`701_portLabels.elkt`, MyNode1 with north port label at y=21):
```
Before fix:  compute_inside_part(pos=(0,0), ...) → small inside_part → height=52
After fix:   compute_inside_part(pos=(-40,21), ...) → correct inside_part → height=88
Java:        height=88
```

**Fix**: Collect `label_positions` alongside `label_sizes` from port contexts and pass the real positions to `compute_inside_part()`.

### Bug 3: Edgeless Root Ports Not Treated as External (ElkGraphImporter)

**File**: `plugins/.../layered/graph/transform/elk_graph_importer.rs`

**Root cause**: `check_external_ports()` only recognized ports with edges as external ports. Root-level ports with no edges to internal nodes were never converted to external port dummies.

**Symptom**: Root-level edgeless ports were not positioned by the layered pipeline, remaining at their input coordinates. This caused the 70-diff regression in the QA OAD model.

**Note**: This is not a Rust porting bug — Java ELK has the identical limitation. It was never discovered because no standard test model has root-level edgeless ports.

**Fix**: Added a fallback check: when no edge-based external ports are found, treat all root ports as external if `portConstraints >= FIXED_SIDE`. (Identical to `custom/external-port` branch fix.)

---

## Changed Files

| File | Description |
|------|-------------|
| `.../nodespacing/node_dimension_calculation.rs` | Flag flip: `ELK_NODE_DIM_USE_FULL_PROCESS` (opt-in) → `ELK_NODE_DIM_SKIP_FULL_PROCESS` (opt-out) |
| `.../nodespacing/internal/algorithm/node_label_cell_creator.rs` | Bug 1 fix: container-based cell existence check |
| `.../nodespacing/internal/algorithm/node_label_and_size_utilities.rs` | Bug 2 fix: real label positions in `compute_inside_part()` |
| `.../layered/graph/transform/elk_graph_importer.rs` | Bug 3 fix: edgeless root port external port treatment + compound minimum size guard comment |
| `.../layered/intermediate/label_and_node_size_processor.rs` | Step 2 disabled when full process active; self-loop node handling |
| `.../layered/tests/.../port_label_placement_variants_test.rs` | Test relaxation: directional assert → no-overlap assert |
| `.../graph.json/tests/all/edge_coords_test.rs` | Expected port label y-coordinates updated |
| `.../graph.json/tests/all/root_external_ports_test.rs` | 9 edgeless root port tests (from `custom/external-port`) |
| `.../graph.json/tests/all/edgeless_hierarchy_integration_test.rs` | 6 integration tests for edgeless hierarchical model |
| `.../alg.common/tests/all/cellsystem_process_test.rs` | 2 new CellSystem bug regression tests |

## Test Coverage

### CellSystem Bug Tests (`cellsystem_process_test.rs`)

| Test | Bug | Verification |
|------|-----|-------------|
| `cellsystem_multi_label_same_location_not_overwritten` | #1 | Two labels at same position both placed, second below first |
| `cellsystem_fixed_port_label_inside_part_uses_real_position` | #2 | Node height accommodates north port label extension |

### Edgeless Root Port Tests (`root_external_ports_test.rs`)

| Test | Bug | Verification |
|------|-----|-------------|
| `root_ext_ports_west_distributed` | #3 | WEST ports Y coords distinct |
| `root_ext_ports_south_distributed` | #3 | SOUTH ports X coords distinct |
| `root_ext_ports_north_distributed` | #3 | NORTH ports X coords distinct |
| `root_ext_ports_east_centered` | #3 | EAST port Y within bounds |
| `root_ext_ports_all_four_sides` | #3 | All 4 sides distributed |
| `root_ext_ports_with_domain_edges` | #3 | Works with child edges present |
| `root_ext_ports_fixed_order` | #3 | Order preserved |
| `root_ext_ports_multiple_children` | #3 | Works with multiple children |
| `root_ext_ports_single_per_side` | #3 | Single port per side OK |

### Edgeless Hierarchy Integration Tests (`edgeless_hierarchy_integration_test.rs`)

| Test | Bug | Verification |
|------|-----|-------------|
| `edgeless_hierarchy_layout_completes` | All | Edgeless hierarchical model layouts without panic |
| `edgeless_hierarchy_domain_node_sized` | All | Domain compound node has reasonable dimensions |
| `edgeless_hierarchy_multi_label_leaf_positioned` | #1 | Two labels at same location both positioned, second below first |
| `edgeless_hierarchy_domain_ports_distributed` | #3 | WEST/SOUTH ports distributed along their sides |
| `edgeless_hierarchy_leaf_nodes_have_distinct_positions` | All | Leaf nodes placed at distinct positions |
| `edgeless_hierarchy_deterministic` | All | Layout produces identical output across runs |

```bash
# Run bug regression tests
cargo test -p org-eclipse-elk-alg-common cellsystem_process
cargo test -p org-eclipse-elk-graph-json root_external_ports
cargo test -p org-eclipse-elk-graph-json edgeless_hierarchy

# Full verification
cargo test --workspace
cargo clippy --workspace --all-targets
MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models tests/model_parity_full
SKIP_JAVA_TRACE=true sh scripts/run_full_trace_parity.sh external/elk-models tests/model_parity_full
```

## Why These Bugs Were Not Found Earlier

The `process()` CellSystem code path had **zero test coverage**:

- `process_node()` was the default path and passed all 1988 model parity tests
- `process()` was opt-in via environment variable and never enabled during CI or parity testing
- The existing models were sufficient to expose the bugs — they just never ran through the `process()` path

The QA customer OAD model triggered the switch to `process()` because `process_node()` could not resolve the 70-diff port sizing issue. This immediately revealed the three latent bugs.

---

# elk-rs Custom Features — elk-live

## Overview

This document describes the **elk-live** custom feature: a standalone web demonstrator for elk-rs that replaces the original Sprotty-based [elk-live](https://rtsys.informatik.uni-kiel.de/elklive/) with a lightweight Vite + Monaco + SVG implementation powered by the elk-rs WASM engine.

The original elk-live (Java/Sprotty) is preserved as a reference submodule at `external/elk-live`.

## Branch and Version

| Item | Value |
|------|-------|
| Feature branch | `custom/elk-live` |
| Base | `main` (`v0.11.0` — ELK Java 1:1 parity) |
| Package | `elk-rs-live@0.11.0` (private, not published) |
| Reference submodule | `external/elk-live` → [kieler/elk-live](https://github.com/kieler/elk-live) |

---

## Feature: elk-live Demonstrator

### Description

A standalone web application that provides two main views:

1. **Interactive Editor** (`editor.html`): ELKT/JSON editor with live layout preview. Supports mode switching (elkt↔json), URL-based model sharing via LZ-string compression, and a "Link to this model" feature.

2. **Examples Browser** (`examples.html`): Sidebar navigation of all elk-models examples (`.elkt` files with `elkex:` annotations), with live editor, SVG diagram, and Markdown description panel.

Both views share a common SVG renderer with Sprotty-compatible pan/zoom and per-element animation.

### Architecture

```
packages/elk-live/
├── src/
│   ├── editor.ts              # Interactive editor entry point
│   ├── examples.ts            # Examples browser entry point
│   ├── index.ts               # Landing page
│   ├── common/
│   │   ├── dark-mode.ts       # Dark mode toggle (localStorage)
│   │   ├── elkt-language.ts   # Monaco ELKT language definition
│   │   └── url-params.ts      # URL parameter parsing
│   ├── elk/
│   │   ├── elk-layout.ts      # WASM layout interface
│   │   └── elk-types.ts       # ELK JSON type definitions
│   ├── elkt/
│   │   └── parser.ts          # ELKT text → ELK JSON parser
│   ├── elkex/
│   │   └── parser.ts          # Example file annotation parser
│   └── render/
│       └── svg-renderer.ts    # SVG renderer with pan/zoom/animation
├── styles/
│   ├── common.css             # Shared CSS (navbar, footer, panes, dark mode)
│   └── diagram.css            # SVG diagram styling (nodes, edges, labels)
├── test/
│   ├── elkt-parser.test.ts    # ELKT parser unit tests
│   ├── elkex-parser.test.ts   # Example parser unit tests
│   └── all-examples-wasm.test.ts  # E2E: parse + layout + parity check
├── editor.html                # Interactive editor page
├── examples.html              # Examples browser page
├── index.html                 # Landing page
├── setup.mjs                  # WASM file copy script
├── vite.config.ts             # Vite build configuration
└── vitest.config.ts           # Test configuration
```

### Key Components

#### SVG Renderer (`src/render/svg-renderer.ts`)

Sprotty-compatible rendering without viewBox:

- **Viewport**: No SVG `viewBox`/`width`/`height` attributes. Root `<g>` uses `transform="scale(s) translate(tx,ty)"` — matches original Sprotty approach for consistent sub-pixel stroke rendering across different container sizes.
- **Pan**: Mouse drag adjusts `translate` by `dx/scale, dy/scale`.
- **Zoom**: Wheel zoom keeps the point under cursor fixed: `scroll += mx/scale * (1 - 1/factor)`.
- **Animation**: Per-element move (SVG `transform` attribute interpolation) + fade-in (SVG `opacity` attribute interpolation), 300ms ease-in-out. `animId` counter cancels in-flight animations on re-render.
- **Element tracking**: Every logical element wrapped in `<g data-elk-id="...">` for position snapshot/restore across re-renders.

#### ELKT Parser (`src/elkt/parser.ts`)

Full tokenizer + recursive descent parser:

- Tokenizer: whitespace, line/block comments, strings, numbers, booleans, null, keywords, identifiers (with dots for qualified IDs, `^` escape)
- Parser: nodes, ports, edges (with optional ID prefix), labels, layout options, layout sections (`size:`, `position:`), nested hierarchies
- ID qualification: local IDs qualified with parent scope (e.g., `parent$child$port`) for global uniqueness
- Edge endpoint dot notation: `n1.p1` → `n1$p1` (port reference)
- Defaults: nodes 30x30, ports 5x5, labels `text.length * 9` x 16 (matches Java `ElkGraphDiagramGenerator.applyDefaults`)

#### Example Parser (`src/elkex/parser.ts`)

Parses `elkex:` annotations from `.elkt` example files:

- Sections: `category`, `label`, `doc`, `graph`
- Builds hierarchical category tree for sidebar navigation
- Markdown documentation rendered via Showdown

### Setup

```bash
cd packages/elk-live
npm install
node setup.mjs      # copies WASM files from ../../target/wasm-dist/
npm run dev          # starts Vite dev server
```

`setup.mjs` copies the WASM glue files (`org_eclipse_elk_wasm.js`, `org_eclipse_elk_wasm_bg.wasm`, `org_eclipse_elk_wasm.d.ts`) from the workspace build output into `src/wasm/`.

### Build

```bash
npm run build        # produces dist/ with editor, examples, index pages
npm run test         # runs vitest (parser unit tests + E2E parity)
```

Build-time version injection: `__APP_VERSION__` is defined from `package.json` version via Vite `define` — no hardcoded version strings in HTML.

### Differences from Original elk-live

| Aspect | Original (Sprotty) | elk-rs (this) |
|--------|-------------------|---------------|
| Layout engine | Java ELK via WebSocket | elk-rs WASM (client-side) |
| Rendering | Sprotty framework (TypeScript) | Lightweight SVG renderer (~400 LOC) |
| Editor | Monaco | Monaco |
| Bundler | Webpack | Vite |
| Server | Eclipse Jetty + WebSocket | Static files only |
| Animation | Sprotty moveModule/fadeModule | SVG attribute interpolation (compatible) |
| Viewport | `scale(s) translate(tx,ty)` on root `<g>` | Same approach (no viewBox) |
| Dark mode | CSS filter invert | Same approach |
| Examples | Server-side file listing | Vite `import.meta.glob` at build time |

### Changed Files

| File | Description |
|------|-------------|
| `.gitmodules` | Added `external/elk-live` submodule reference |
| `external/elk-live` | Reference submodule (original Sprotty-based elk-live) |
| `packages/elk-live/` | All files listed in Architecture section above |

### Test Coverage

| Scope | Tests | File |
|-------|-------|------|
| ELKT parser unit | tokenizer + parser cases | `test/elkt-parser.test.ts` |
| Example parser unit | annotation parsing + category tree | `test/elkex-parser.test.ts` |
| E2E parity | parse → NAPI layout → compare with model parity reference | `test/all-examples-wasm.test.ts` |
