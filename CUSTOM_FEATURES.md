# elk-rs Custom Features

## Overview

This document describes **elk-rs-specific custom features** that do not exist in the original ELK Java (v0.11.0).

elk-rs is a 1:1 port of ELK Java, and the `main` branch maintains 100% parity with Java. Custom features are developed on separate branches forked from `main` and merged into the `custom/0.11.0` integration branch.

Four custom features are currently implemented:

| # | Feature | Branch | Description |
|---|---------|--------|-------------|
| 1 | ignoreEdgeInLayer | `custom/ignore-edge-in-layer` | Bypasses layer separation for specific edges, allowing same-layer placement |
| 2 | In-Layer Edge Routing | `custom/in-layer-edge-routing` | Enables edge routing between FIRST/LAST_SEPARATE nodes within the same layer |
| 3 | elk-live Demonstrator | `custom/elk-live` | Standalone web demo (Vite + Monaco + WASM) replacing the original Sprotty-based elk-live |
| 4 | Root External Port Placement | `custom/external-port` | Distributes edgeless root-level ports along their assigned sides (Java ELK improvement) |

## Branch and Version

| Item | Value |
|------|-------|
| Integration branch | `custom/0.11.0` |
| Tag | `v0.11.0-ext.2` |
| Cargo version | `0.11.0-ext.2` |
| npm version | `elk-rs@0.11.0-ext.2` |
| Base | `main` (`v0.11.0` — ELK Java 1:1 parity) |

Feature development branches:

| Feature | Branch |
|---------|--------|
| ignoreEdgeInLayer | `custom/ignore-edge-in-layer` |
| In-Layer Edge Routing | `custom/in-layer-edge-routing` |
| elk-live Demonstrator | `custom/elk-live` |

For version/tag/branch management rules, see `VERSIONING.md` §1 (Extension Releases) and §3 (Branch and Tag Policy).

---

## Feature 1: ignoreEdgeInLayer

### Branch

`custom/ignore-edge-in-layer`

### Description

Setting `ignoreEdgeInLayer: true` on an edge allows its source and target nodes to be **placed in the same layer**. Normally, the layered algorithm places source and target nodes in different layers when connected by an edge. This option removes the layer separation constraint.

**Use case**: Useful for grid-style layouts where connections between nodes within the same column (layer) need to be expressed. For example, mutual references between components at the same hierarchy level can be displayed without forcing layer separation.

### Layout Option

| Field | Value |
|-------|-------|
| Property ID | `org.eclipse.elk.alg.layered.layering.ignoreEdgeInLayer` |
| Type | `bool` |
| Default | `false` |
| Applies to | Edge |
| JSON key | `ignoreEdgeInLayer` |

### How It Works

1. **NetworkSimplex delta=0**: When an edge's `ignoreEdgeInLayer` is `true`, the NetworkSimplex layerer sets the edge's `delta` to 0. `delta` is the minimum layer distance between source and target — 0 allows same-layer placement. The weight (priority) is preserved to participate in layer assignment optimization.

2. **Same-layer placement**: With `delta=0`, the NetworkSimplex algorithm can place source and target in the same layer. If no other edge constraints prevent it, both nodes end up in the same layer.

3. **Automatic EAST→WEST reversal**: After layer assignment, any `ignoreEdgeInLayer` edge in the same layer going from an EAST port to a WEST port is automatically reversed. This ensures correct visual direction during subsequent edge routing phases.

### Usage Example

Add `"ignoreEdgeInLayer": true` to the edge's `properties`:

```json
{
  "id": "root",
  "properties": {
    "algorithm": "layered",
    "strategy": "NETWORK_SIMPLEX",
    "edgeRouting": "OTHOGONAL"
  },
  "children": [
    {
      "id": "n1", "width": 30, "height": 30,
      "properties": { "portConstraints": "FIXED_SIDE" },
      "ports": [
        { "id": "n1_east", "properties": { "side": "EAST" } },
        { "id": "n1_west", "properties": { "side": "WEST" } }
      ]
    },
    {
      "id": "n2", "width": 30, "height": 30,
      "properties": { "portConstraints": "FIXED_SIDE" },
      "ports": [
        { "id": "n2_east", "properties": { "side": "EAST" } },
        { "id": "n2_west", "properties": { "side": "WEST" } }
      ]
    },
    {
      "id": "n3", "width": 30, "height": 30,
      "properties": { "portConstraints": "FIXED_SIDE" },
      "ports": [
        { "id": "n3_west", "properties": { "side": "WEST" } }
      ]
    }
  ],
  "edges": [
    {
      "id": "e_normal", "sources": ["n1_east"], "targets": ["n3_west"]
    },
    {
      "id": "e_same_layer", "sources": ["n1_east"], "targets": ["n2_west"],
      "properties": { "ignoreEdgeInLayer": true }
    }
  ]
}
```

In this example, `e_normal` places n1 and n3 in different layers, while `e_same_layer` allows n1 and n2 to be placed in the same layer.

### Changed Files

| File | Change |
|------|--------|
| `plugins/org.eclipse.elk.alg.layered/src/.../options/layered_options.rs` | `LAYERING_IGNORE_EDGE_IN_LAYER` property definition |
| `plugins/org.eclipse.elk.alg.layered/src/.../options/layered_meta_data_provider.rs` | Property registration (target: edges) |
| `plugins/org.eclipse.elk.alg.layered/src/.../p2layers/network_simplex_layerer.rs` | delta=0 assignment + EAST→WEST reversal logic |
| `plugins/org.eclipse.elk.alg.layered/tests/p2_layers/ignore_edge_in_layer_test.rs` | 3 unit tests |
| `plugins/org.eclipse.elk.alg.layered/tests/models/ignore_edge_in_layer_integration_test.rs` | 3 integration tests |
| `plugins/org.eclipse.elk.graph.json/tests/fixtures/*ignoreEdgeInLayer*.elk.json` | 10 fixture files |

---

## Feature 2: In-Layer Edge Routing

### Branch

`custom/in-layer-edge-routing`

### Description

Enables **in-layer edge routing** between nodes with `layerConstraint: FIRST_SEPARATE` or `LAST_SEPARATE`.

In the original ELK Java, connecting an in-layer edge to FIRST_SEPARATE/LAST_SEPARATE nodes causes a validation failure (panic) or unnecessary dummy node creation. This feature consists of 4 sub-changes (B-1 through B-4) that properly handle in-layer edges across intermediate processors.

**Use case**: Needed when expressing connections between nodes pinned to the first or last layer (e.g., I/O ports, domain boundaries) in a hierarchical layout. It works automatically with existing `layerConstraint` and `portConstraints` combinations — no additional layout option is required.

### Sub-Changes

#### B-1: In-layer edge reversal for FIRST/LAST_SEPARATE nodes

**File**: `edge_and_layer_constraint_edge_reverser.rs`

Previously, edges targeting FIRST_SEPARATE nodes or originating from LAST_SEPARATE nodes were unconditionally blocked. After this change, reversal is allowed when **both source and target are NodeType::Normal** and the direction is **EAST→WEST**.

- `can_reverse_outgoing_edge`: For FIRST_SEPARATE target nodes, allows reversal if both source/target are NORMAL with source EAST and target WEST
- `can_reverse_incoming_edge`: For LAST_SEPARATE source nodes, allows reversal under the same conditions
- Preserves original behavior for non-NORMAL node types (ExternalPort, Label, etc.) and non-EAST→WEST directions

#### B-2: Edge constraint validation relaxation

**File**: `layer_constraint_preprocessor.rs`

The original `ensure_no_inacceptable_edges` function panicked on any incoming edge to FIRST_SEPARATE or outgoing edge from LAST_SEPARATE. This validation is disabled via a **feature flag** (`USE_ENSURE_NO_INACCEPTABLE_EDGES = false`) to allow in-layer edges.

The original validation functions are preserved with `#[allow(dead_code)]` and can be re-enabled if needed.

#### B-3: Dummy node skip for same-layer ports

**File**: `inverted_port_processor.rs`

During inverted port processing, long-edge dummy nodes were previously created for all inverted ports. After this change, dummy creation is **skipped when source and target are in the same layer**.

- EAST input port: skips dummy if the source is in the same layer
- WEST output port: skips dummy if the target is in the same layer
- Same-layer detection: `Arc::ptr_eq` pointer comparison on layer references

#### B-4: Dedicated EXTERNAL_PORT layer separation

**File**: `layer_constraint_postprocessor.rs`

Previously, all FIRST_SEPARATE/LAST_SEPARATE nodes were placed in a single separate layer. After this change, **EXTERNAL_PORT nodes are separated into dedicated layers** to prevent mixing with regular nodes.

Layer order:

```
[first_external_port] → [first_separate] → [normal layers...] → [last_separate] → [last_external_port]
```

- EXTERNAL_PORT-typed FIRST/LAST_SEPARATE nodes → dedicated external port layers
- Other FIRST/LAST_SEPARATE nodes → original separate layers

### Usage Example

Works automatically with `layerConstraint` and port configuration — no additional option needed:

```json
{
  "id": "root",
  "properties": { "algorithm": "layered", "edgeRouting": "OTHOGONAL" },
  "children": [
    {
      "id": "n1", "width": 30, "height": 30,
      "properties": { "layerConstraint": "FIRST_SEPARATE", "portConstraints": "FIXED_SIDE" },
      "ports": [
        { "id": "n1_mmi", "properties": { "side": "EAST" } },
        { "id": "n1_rmi", "properties": { "side": "EAST" } }
      ]
    },
    {
      "id": "n2", "width": 30, "height": 30,
      "properties": { "layerConstraint": "FIRST_SEPARATE", "portConstraints": "FIXED_SIDE" },
      "ports": [
        { "id": "n2_mmi", "properties": { "side": "EAST" } },
        { "id": "n2_rsi", "properties": { "side": "WEST" } }
      ]
    },
    {
      "id": "n4", "width": 30, "height": 30,
      "properties": { "portConstraints": "FIXED_SIDE" },
      "ports": [
        { "id": "n4_msi", "properties": { "side": "WEST" } }
      ]
    }
  ],
  "edges": [
    { "id": "e01", "sources": ["n1_mmi"], "targets": ["n4_msi"] },
    { "id": "e02", "sources": ["n2_mmi"], "targets": ["n4_msi"] },
    { "id": "e07", "sources": ["n1_rmi"], "targets": ["n2_rsi"] }
  ]
}
```

In this example, n1 and n2 are both `FIRST_SEPARATE` and placed in the same layer. `e07` is an in-layer edge from n1 (EAST) to n2 (WEST), correctly handled by B-1 through B-4.

### Changed Files

| File | Change |
|------|--------|
| `plugins/org.eclipse.elk.alg.layered/src/.../intermediate/edge_and_layer_constraint_edge_reverser.rs` | B-1: Allow in-layer edge reversal for FIRST/LAST_SEPARATE |
| `plugins/org.eclipse.elk.alg.layered/src/.../intermediate/layer_constraint_preprocessor.rs` | B-2: Edge validation relaxation (feature flag) |
| `plugins/org.eclipse.elk.alg.layered/src/.../intermediate/inverted_port_processor.rs` | B-3: Same-layer dummy skip |
| `plugins/org.eclipse.elk.alg.layered/src/.../intermediate/layer_constraint_postprocessor.rs` | B-4: Dedicated EXTERNAL_PORT layers |
| `plugins/org.eclipse.elk.alg.layered/tests/intermediate/edge_and_layer_constraint_edge_reverser_test.rs` | 4 unit tests |
| `plugins/org.eclipse.elk.alg.layered/tests/models/in_layer_edge_routing_integration_test.rs` | 3 integration tests |
| `plugins/org.eclipse.elk.graph.json/tests/fixtures/01_*.elk.json` ~ `08_*.elk.json` | 8 fixture files (direction/domain/group variants) |

---

## Feature 3: elk-live Demonstrator

### Branch

`custom/elk-live`

### Description

A standalone web application that replaces the original Sprotty-based [elk-live](https://rtsys.informatik.uni-kiel.de/elklive/) with a lightweight Vite + Monaco + SVG implementation powered by the elk-rs WASM engine. The original elk-live (Java/Sprotty) is preserved as a reference submodule at `external/elk-live`.

Two main views:

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

---

## Feature 4: Root External Port Placement

### Branch

`custom/external-port`

### Description

When external ports are declared on the root node without edges connecting them to internal child nodes, the layered algorithm now correctly distributes them along their assigned sides. This fixes a design limitation present in both Java ELK and elk-rs.

**Before (buggy):**
```
Root: w=640, h=420
  Port si0: x=0, y=0       <- All 3 WEST at (0, 0)
  Port si1: x=0, y=0
  Port si2: x=0, y=0
  Port clk0: x=0, y=420    <- All 3 SOUTH at (0, 420)
  Port clk1: x=0, y=420
  Port clk2: x=0, y=420
```

**After (fixed):**
```
Root: w=640, h=420
  Port si0: x=0, y=~100    <- WEST distributed vertically
  Port si1: x=0, y=~200
  Port si2: x=0, y=~300
  Port clk0: x=~160, y=420 <- SOUTH distributed horizontally
  Port clk1: x=~320, y=420
  Port clk2: x=~480, y=420
```

### Root Cause

`ElkGraphImporter::check_external_ports()` only recognized ports with edges (`externalPortEdges > 0`) as external ports. Edgeless root ports were never converted to external port dummies, leaving their coordinates at the input default (0, 0).

This is not a Rust porting bug — Java ELK has the identical limitation in `ElkGraphImporter.checkExternalPorts()`. It was never discovered because:

1. All ~1,998 test models in `external/elk-models/` have zero root-level ports
2. Tools like `elk_advanced` pre-process graphs with edge hoisting, providing edges before ELK sees the graph
3. The original elk-live uses a Java WebSocket server that may apply similar preprocessing

### Fix

**File**: `plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/elk_graph_importer.rs`

Added a root-only fallback at the end of `check_external_ports()` (~12 lines):

```rust
// When edge-based detection returns false and the node is root,
// check if the graph has ports with portConstraints >= FIXED_SIDE
// — if so, treat them as external ports to ensure dummy creation.
let is_root = elkgraph.borrow().parent().is_none();
if is_root && !has_external_ports && has_any_ports {
    let port_constraints = elkgraph
        .get_property(CoreOptions::PORT_CONSTRAINTS)
        .unwrap_or(PortConstraints::Undefined);
    if port_constraints.is_side_fixed() {
        has_external_ports = true;
    }
}
```

Once `GraphProperties::ExternalPorts` is set, the existing layered pipeline handles distribution naturally:

1. **Import**: External port dummies are created for each root port
2. **LayerConstraintPreprocessor/Postprocessor**: W/E dummies placed in first/last layers
3. **P3 Crossing Minimization**: Assigns distinct ordering to same-side dummies
4. **P4 Node Placement**: Allocates distinct Y/X positions
5. **HierarchicalPortOrthogonalEdgeRouter**: Handles N/S dummy coordinates

### Differences from Java ELK

| Aspect | Java ELK | elk-rs (this fix) |
|--------|----------|-------------------|
| Edgeless root ports | Dummy not created (bug) | Dummy created (fixed) |
| Ports with edges | Dummy created (OK) | Dummy created (OK) |
| Model parity impact | N/A (no root-port models) | No regression (0 affected models) |
| `portConstraints` check | Not present | Added as fallback |

### Changed Files

| File | Description |
|------|-------------|
| `plugins/.../graph/transform/elk_graph_importer.rs` | Root-only fallback in `check_external_ports()` (~12 lines added) |
| `plugins/org.eclipse.elk.graph.json/tests/all/root_external_ports_test.rs` | 9 new integration tests (new file) |
| `plugins/org.eclipse.elk.graph.json/tests/all/mod.rs` | Module registration (1 line added) |

### Test Coverage

| Test | Scenario | Verification |
|------|----------|-------------|
| `root_ext_ports_west_distributed` | WEST 3 ports, no edges | Y coords distinct + within root height |
| `root_ext_ports_south_distributed` | SOUTH 3 ports, no edges | X coords distinct + within root width |
| `root_ext_ports_north_distributed` | NORTH 3 ports, no edges | X coords distinct + within root width |
| `root_ext_ports_east_centered` | EAST 1 port | Y within root height |
| `root_ext_ports_all_four_sides` | N/S/E/W 2 each | All 4 sides distinct + in range |
| `root_ext_ports_with_domain_edges` | 7 ports + domain with internal edges | Root ports distributed even with child edges |
| `root_ext_ports_fixed_order` | FIXED_ORDER + W 3 ports | Order preserved + distributed |
| `root_ext_ports_multiple_children` | 2 children + 4 ports | Distributed with multiple children |
| `root_ext_ports_single_per_side` | W 1 + E 1 | Single port per side placed correctly |

---

## Cross-Feature Compatibility

All features work independently and can be used together.

- `ignoreEdgeInLayer` can place regular nodes in the same layer while in-layer edges connect `layerConstraint: FIRST_SEPARATE/LAST_SEPARATE` nodes
- Validated by cross-feature integration test (`551be8d`) and combined fixtures (`09_*.ignoreEdgeInLayer.elk.json`, `10_*.ignoreEdgeInLayer.elk.json`)
- elk-live demonstrates all custom features via the interactive editor (ELKT/JSON input with live WASM layout)
- Root external port placement works independently of all other features — it only affects root-level edgeless ports

### Test Coverage Summary

| Scope | Tests | Location |
|-------|-------|----------|
| ignoreEdgeInLayer unit | 3 | `tests/p2_layers/ignore_edge_in_layer_test.rs` |
| ignoreEdgeInLayer integration | 3 | `tests/models/ignore_edge_in_layer_integration_test.rs` |
| In-Layer Edge Routing unit | 4+ | `tests/intermediate/edge_and_layer_constraint_edge_reverser_test.rs` |
| In-Layer Edge Routing integration | 3 | `tests/models/in_layer_edge_routing_integration_test.rs` |
| Cross-feature integration | 1 | commit `551be8d` |
| Fixture layout tests | 18 | `tests/fixtures/*.elk.json` + `.layout.json` |
| elk-live ELKT parser | unit tests | `packages/elk-live/test/elkt-parser.test.ts` |
| elk-live example parser | unit tests | `packages/elk-live/test/elkex-parser.test.ts` |
| elk-live E2E parity | layout parity | `packages/elk-live/test/all-examples-wasm.test.ts` |
| Root external port integration | 9 | `tests/all/root_external_ports_test.rs` |
