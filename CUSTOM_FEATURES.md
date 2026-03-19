# elk-rs Custom Features — external-port

## Overview

This document describes the **external-port** custom feature: root-level external port placement support for the layered algorithm. Edgeless ports declared directly on the root node are now properly distributed along their assigned sides, fixing a design limitation present in both Java ELK and elk-rs.

The original elk-live (`external/elk-live/`) was used as a visual reference to identify the bug — ports clustering at (0, 0) instead of distributing along N/S/E/W sides.

## Branch and Version

| Item | Value |
|------|-------|
| Feature branch | `custom/external-port` |
| Base | `main` (`v0.11.0` — ELK Java 1:1 parity) |
| Java ELK status | Same bug exists in Java ELK (intentional divergence) |
| Reference submodule | `external/elk-live` — used to verify visual port placement |

---

## Feature: Root-Level External Port Placement

### Description

When external ports are declared on the root node without edges connecting them to internal child nodes, the layered algorithm now correctly distributes them along their assigned sides.

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

```rust
// Before fix — only edge-connected ports trigger dummy creation
if external_port_edges > 0 {
    has_external_ports = true;
}
```

This is not a Rust porting bug — Java ELK has the identical limitation in `ElkGraphImporter.checkExternalPorts()`. It was never discovered because:

1. All ~1,998 test models in `external/elk-models/` have zero root-level ports
2. Tools like `elk_advanced` pre-process graphs with edge hoisting, providing edges before ELK sees the graph
3. The original elk-live uses a Java WebSocket server that may apply similar preprocessing

### Fix

**File**: `plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/elk_graph_importer.rs`

Added a fallback at the end of `check_external_ports()` (~10 lines):

```rust
// When edge-based detection returns false, check if the graph has
// ports with portConstraints >= FIXED_SIDE — if so, treat them as
// external ports to ensure dummy creation.
if !has_external_ports && has_any_ports {
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

### Architecture

```
elk_graph_importer.rs
  └─ check_external_ports()
       ├─ Edge-based detection (existing)          ← ports with edges
       └─ Fallback: portConstraints-based (NEW)    ← edgeless ports
            └─ has_external_ports = true
                 └─ GraphProperties::ExternalPorts set
                      └─ transform_external_port() called per port
                           └─ External port dummies created
                                └─ Standard layered pipeline distributes them
```

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
| `plugins/.../graph/transform/elk_graph_importer.rs` | Fallback in `check_external_ports()` (~10 lines added) |
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

```bash
# Run tests
cargo test -p org-eclipse-elk-graph-json -- root_ext_ports

# Full workspace verification
cargo test --workspace
cargo clippy --workspace --all-targets
```

### Related Documents

| Document | Location |
|----------|----------|
| Bug report | `elk-rs-qa/BUG_REPORT_external_port_placement.md` |
| Bug specification | `elk-rs-qa/BUG_SPEC_external_port_placement.md` |
| Fix plan | `FIX_PLAN_external_port_placement.md` |
| Analysis report | `elk-rs-qa/ANALYSIS_REPORT_external_port_placement.md` |
| Test fixtures | `elk-rs-qa/fixtures/20_*.json`, `elk-rs-qa/fixtures/21_*.json` |
