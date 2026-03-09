# elk-rs Custom Features

## Overview

This document describes **elk-rs-specific custom features** that do not exist in the original ELK Java (v0.11.0).

elk-rs is a 1:1 port of ELK Java, and the `main` branch maintains 100% parity with Java. Custom features are developed on separate branches forked from `main` and merged into the `custom/0.11.0` integration branch.

Two custom features are currently implemented:

| # | Feature | Branch | Description |
|---|---------|--------|-------------|
| 1 | ignoreEdgeInLayer | `custom/ignore-edge-in-layer` | Bypasses layer separation for specific edges, allowing same-layer placement |
| 2 | In-Layer Edge Routing | `custom/in-layer-edge-routing` | Enables edge routing between FIRST/LAST_SEPARATE nodes within the same layer |

## Branch and Version

| Item | Value |
|------|-------|
| Integration branch | `custom/0.11.0` |
| Tag | `v0.11.0-ext.1` |
| Cargo version | `0.11.0-ext.1` |
| npm version | `elk-rs@0.11.0-ext.1` |
| Base | `main` (`v0.11.0` — ELK Java 1:1 parity) |

Feature development branches:

| Feature | Branch |
|---------|--------|
| ignoreEdgeInLayer | `custom/ignore-edge-in-layer` |
| In-Layer Edge Routing | `custom/in-layer-edge-routing` |

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

## Cross-Feature Compatibility

Both features work independently and can be used together.

- `ignoreEdgeInLayer` can place regular nodes in the same layer while in-layer edges connect `layerConstraint: FIRST_SEPARATE/LAST_SEPARATE` nodes
- Validated by cross-feature integration test (`551be8d`) and combined fixtures (`09_*.ignoreEdgeInLayer.elk.json`, `10_*.ignoreEdgeInLayer.elk.json`)

### Test Coverage Summary

| Scope | Tests | Location |
|-------|-------|----------|
| ignoreEdgeInLayer unit | 3 | `tests/p2_layers/ignore_edge_in_layer_test.rs` |
| ignoreEdgeInLayer integration | 3 | `tests/models/ignore_edge_in_layer_integration_test.rs` |
| In-Layer Edge Routing unit | 4+ | `tests/intermediate/edge_and_layer_constraint_edge_reverser_test.rs` |
| In-Layer Edge Routing integration | 3 | `tests/models/in_layer_edge_routing_integration_test.rs` |
| Cross-feature integration | 1 | commit `551be8d` |
| Fixture layout tests | 18 | `tests/fixtures/*.elk.json` + `.layout.json` |
